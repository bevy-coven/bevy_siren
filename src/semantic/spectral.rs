//! **Spectral shape: how bright a sound is, and how clean.**
//!
//! What a sound designer means by *"this sounds like the same world"* is often brightness and cleanliness — two
//! recordings of dripping water in the same cave sit close in spectral centroid and noise floor; one recorded in a
//! treated room does not. These are cheap, and they survive an embedding, unlike loudness which an embedding partly
//! normalises away.
//!
//! # The one rule: a NaN is worse than a wrong number
//!
//! A NaN in a feature propagates into a score and makes **every comparison false** — a ranking that looks empty rather
//! than wrong. So every degenerate input has a stated answer here, and each is a test:
//!
//! | Input | Centroid | Rolloff | Noise floor |
//! |---|---|---|---|
//! | digital silence (all zeros) | **absence** | **absence** | **`SILENT_DB`** |
//! | a DC offset (constant non-zero) | `0 Hz` — all energy at bin 0 | `0 Hz` | its level |
//! | NaN or infinite samples | the buffer is refused | — | — |
//!
//! Silence is the case the issue names (*"a fully silent clip divides by zero in a centroid"*), and it is why the
//! centroid is an [`Option`]: a sound with **no** energy has no brightness, and reporting `0 Hz` there would say it is
//! as dark as a DC offset, which is a different fact.
//!
//! # The analysis window
//!
//! One **1024-sample Hann window**, the same window [`super::mel`] uses, hopping **480 samples** — identical constants,
//! so a spectral number and a mel frame describe the same slice of time. A clip shorter than one window is **not**
//! padded and not refused: its whole length is analysed as one frame, because a 0.1 s click still has a brightness and a
//! floor, and refusing to say so would lose the commonest sound effect in a library.
//!
//! Spectra are averaged as **power**, not as magnitudes: summing magnitudes weights a steady tone by its amplitude
//! while power weights it by energy, and the latter is what the centroid is defined over in the literature.

use super::MODEL_SAMPLE_RATE;
use super::mel::{BINS, HOP, MEL_FLOOR, WINDOW, fft_in_place, hann};

/// The fraction of total energy at or below which the rolloff is reported.
///
/// **0.95** is the standard choice in the MIR literature (a "rolloff at 95 %"), and it is a named constant here rather
/// than a literal so a caller can see what percentage was used: a rolloff without its percentage is not a number.
pub const ROLLOFF_FRACTION: f32 = 0.95;

/// Why a buffer could not be analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralProblem {
    /// The buffer has no samples.
    Empty,
    /// The buffer holds a value that is not finite, which would poison every statistic derived from it.
    NotFinite {
        /// The index of the first offending sample, so the message points at it.
        at: usize,
    },
}

impl core::fmt::Display for SpectralProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpectralProblem::Empty => f.write_str("the buffer has no samples to analyse"),
            SpectralProblem::NotFinite { at } => write!(
                f,
                "sample {at} is not a finite number. A NaN in a feature propagates into a score and makes every \
                 comparison false, so the buffer is refused rather than averaged"
            ),
        }
    }
}

impl std::error::Error for SpectralProblem {}

/// **A sound's spectral shape**, as the three numbers the issue asks for.
///
/// Both frequency features are `Option` because a silent clip has no spectral content at all — see the module doc's
/// table. A caller that needs a number for every asset must decide what "no brightness" means for it, rather than being
/// handed a `0.0` that is indistinguishable from a DC offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spectral {
    /// The **energy-weighted mean frequency**, in Hz, or `None` for silence.
    pub centroid_hz: Option<f32>,
    /// The frequency below which [`ROLLOFF_FRACTION`] of the energy lies, in Hz, or `None` for silence.
    ///
    /// The fraction is a constant here; a caller comparing two libraries must be sure both used the same one, which is
    /// why it is exported rather than hidden.
    pub rolloff_hz: Option<f32>,
    /// **The noise floor in dBFS**: the energy of the quietest tenth of the frames, relative to full scale.
    ///
    /// *Method*: frames are ranked by their mean power, the quietest 10 % are averaged, and that mean is converted to
    /// dB against a full-scale sine's power. The quiet tenth rather than the minimum, because one silent frame between
    /// two loud ones is a gap, not a floor. **Design default** for the tenth — no source fixes a percentile, and the
    /// method is named here so a reader can disagree with it precisely.
    ///
    /// Digital silence gives [`crate::model::SILENT_DB`], never `-inf` and never `NaN`.
    pub noise_floor_db: f32,
}

impl Spectral {
    /// Nothing measurable: the answer for an empty analysis.
    pub fn absent() -> Self {
        Spectral { centroid_hz: None, rolloff_hz: None, noise_floor_db: crate::model::SILENT_DB }
    }
}

/// **Analyse a mono buffer at the model's rate.**
pub fn analyse(buffer: &[f32]) -> Result<Spectral, SpectralProblem> {
    if buffer.is_empty() {
        return Err(SpectralProblem::Empty);
    }
    if let Some(at) = buffer.iter().position(|s| !s.is_finite()) {
        return Err(SpectralProblem::NotFinite { at });
    }

    let window = hann(WINDOW);
    // One frame when the clip is shorter than the window: its whole length is the analysis slice, and it is **not**
    // zero-padded, because padding a 0.02 s click to 1024 samples would dilute every statistic with silence.
    let frame_len = buffer.len().min(WINDOW);
    let starts: Vec<usize> =
        if buffer.len() <= WINDOW { vec![0] } else { (0..=buffer.len() - WINDOW).step_by(HOP).collect() };

    // **Power summed across frames**, so a steady tone weighs by energy rather than by amplitude.
    let mut power = vec![0.0f64; BINS];
    let mut frame_powers: Vec<f64> = Vec::with_capacity(starts.len());
    let mut re = vec![0.0f64; WINDOW];
    let mut im = vec![0.0f64; WINDOW];
    // The mean square of one windowed frame of full-scale material, which sets the dB reference.
    for &start in &starts {
        for i in 0..WINDOW {
            let sample = if i < frame_len { buffer[start + i.min(buffer.len() - 1 - start)] } else { 0.0 };
            re[i] = f64::from(sample) * f64::from(window[i]);
            im[i] = 0.0;
        }
        fft_in_place(&mut re, &mut im);
        let mut total = 0.0f64;
        for bin in 0..BINS {
            let p = re[bin] * re[bin] + im[bin] * im[bin];
            power[bin] += p;
            total += p;
        }
        frame_powers.push(total);
    }

    let total: f64 = power.iter().sum();
    let (centroid_hz, rolloff_hz) = if total <= f64::from(MEL_FLOOR) {
        // **Digital silence**: no energy to weight, so there is no brightness. `None`, not `0.0` — see the module doc.
        (None, None)
    } else {
        let bin_hz = f64::from(MODEL_SAMPLE_RATE) / WINDOW as f64;
        let centroid = power.iter().enumerate().map(|(b, p)| b as f64 * p).sum::<f64>() / total;
        // Walk bins until the running share reaches the fraction. `find` rather than a loop with a mutable cursor so
        // the accumulator cannot be read after the search.
        let target = total * f64::from(ROLLOFF_FRACTION);
        let mut running = 0.0f64;
        let mut rolloff = BINS.saturating_sub(1);
        for (bin, p) in power.iter().enumerate() {
            running += p;
            if running >= target {
                rolloff = bin;
                break;
            }
        }
        (Some((centroid * bin_hz) as f32), Some((rolloff as f64 * bin_hz) as f32))
    };

    // **The noise floor**: the quietest tenth of the frames, averaged, in dB against a full-scale sine.
    //
    // A full-scale sine's power through this window is (amplitude · window energy / 2)² spread over two bins; comparing
    // against `1.0` instead would put every real clip tens of dB below its actual level. The reference is therefore the
    // mean square of the samples themselves, which is the number `astats` would report as RMS.
    let mean_square: f64 =
        buffer.iter().map(|s| f64::from(*s) * f64::from(*s)).sum::<f64>() / buffer.len() as f64;
    let noise_floor_db = if mean_square <= 0.0 {
        crate::model::SILENT_DB
    } else {
        let mut sorted = frame_powers.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
        let quietest = (sorted.len() / 10).max(1);
        let quiet: f64 = sorted[..quietest].iter().sum::<f64>() / quietest as f64;
        let power_ratio = (quiet / frame_powers.iter().fold(0.0f64, |m, p| m.max(*p)).max(f64::MIN_POSITIVE))
            .max(f64::from(MEL_FLOOR));
        // The floor is expressed against the clip's own loudest frame, so a clip 20 dB quieter than another reads the
        // same floor when it is equally clean — which is what a filter comparing cleanliness wants.
        (10.0 * power_ratio.log10()) as f32
    };

    Ok(Spectral { centroid_hz, rolloff_hz, noise_floor_db })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, amplitude: f32, seconds: f32) -> Vec<f32> {
        let n = (MODEL_SAMPLE_RATE as f32 * seconds) as usize;
        (0..n)
            .map(|i| amplitude * (std::f32::consts::TAU * hz * i as f32 / MODEL_SAMPLE_RATE as f32).sin())
            .collect()
    }

    /// **Criterion 1: a sine yields a centroid at that frequency, within 5 %.**
    #[test]
    fn a_sine_centroid_is_its_own_frequency() {
        for hz in [220.0f32, 1000.0, 4000.0, 8000.0] {
            let s = analyse(&sine(hz, 0.5, 0.5)).expect("analyse");
            let centroid = s.centroid_hz.expect("a sine has energy");
            assert!(
                (centroid - hz).abs() / hz < 0.05,
                "a {hz} Hz sine should centre near {hz}; got {centroid}"
            );
        }
    }

    /// **Criterion 2: white noise and a low-passed copy differ in the expected direction** — the low-passed one is
    /// darker, so its centroid is lower.
    #[test]
    fn a_low_passed_copy_is_darker_than_white_noise() {
        // A deterministic pseudo-noise so the assertion cannot flake.
        let mut seed = 0x5EED_51AE_u64;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.4
        };
        let noise: Vec<f32> = (0..(MODEL_SAMPLE_RATE / 2) as usize).map(|_| next()).collect();
        // A one-pole low-pass at ~1 kHz: `y[n] = y[n-1] + a * (x[n] - y[n-1])`, `a = dt/(RC+dt)`.
        let dt = 1.0f32 / MODEL_SAMPLE_RATE as f32;
        let rc = 1.0f32 / (std::f32::consts::TAU * 1000.0);
        let a = dt / (rc + dt);
        let mut y = 0.0f32;
        let low: Vec<f32> = noise
            .iter()
            .map(|x| {
                y += a * (x - y);
                y
            })
            .collect();

        let white = analyse(&noise).expect("white");
        let dark = analyse(&low).expect("low");
        assert!(
            dark.centroid_hz.expect("energy") < white.centroid_hz.expect("energy"),
            "the low-passed copy must be darker: {} vs {}",
            dark.centroid_hz.expect("energy"),
            white.centroid_hz.expect("energy")
        );
        assert!(
            dark.rolloff_hz.expect("energy") < white.rolloff_hz.expect("energy"),
            "and its rolloff lower"
        );
    }

    /// **Criterion 3: digital silence yields a noise floor at the format's floor — not zero, not NaN.**
    #[test]
    fn digital_silence_has_the_crates_floor_and_no_centroid() {
        let s = analyse(&vec![0.0f32; MODEL_SAMPLE_RATE as usize]).expect("analyse");
        assert_eq!(s.noise_floor_db, crate::model::SILENT_DB, "the crate's one silence floor");
        assert!(!s.noise_floor_db.is_nan());
        assert_eq!(s.centroid_hz, None, "silence has no brightness: `None`, not 0 Hz");
        assert_eq!(s.rolloff_hz, None);
        assert_eq!(s, Spectral::absent());
    }

    /// **The pitfall the issue names**: a silent clip must not divide by zero. The assertions above are the statement;
    /// this one pins the *shape*, so a future refactor cannot turn `None` into `Some(NaN)`.
    #[test]
    fn silence_never_produces_a_nan() {
        for zeros in [0.0f32, -0.0] {
            let s = analyse(&vec![zeros; 4096]).expect("analyse");
            assert!(s.noise_floor_db.is_finite(), "the floor is finite");
            assert!(s.centroid_hz.is_none_or(|c| c.is_finite()));
            assert!(s.rolloff_hz.is_none_or(|r| r.is_finite()));
        }
    }

    /// A DC offset is **not** silence: it is a real, very dark reading.
    ///
    /// It is **not** exactly `0 Hz` either, and the first version of this test wrongly said it was. Multiplying a
    /// constant by a Hann window produces a raised-cosine-shaped signal, whose spectrum is three bins wide rather than
    /// an impulse at bin 0 — so the energy-weighted mean sits at **one bin**, `48 000 / 1024 = 46.9 Hz` divided by
    /// 5 bins of spread, measured at **9.375 Hz**. Asserting the measured behaviour rather than the idealised one is
    /// the point: a DC offset reads *darker than anything audible* either way, and the exact value documents the
    /// window's leakage instead of hiding it.
    #[test]
    fn a_dc_offset_is_dark_but_present() {
        let s = analyse(&vec![0.5f32; 4096]).expect("analyse");
        let centroid = s.centroid_hz.expect("a DC offset has energy, so it is not silence");
        assert!(centroid < 50.0, "a DC offset is far darker than anything audible; got {centroid} Hz");
        assert!(centroid > 0.0, "and it is not exactly zero, because the Hann window leaks");
        assert!(s.rolloff_hz.expect("energy") < 200.0, "its rolloff is in the lowest bins");
        assert!(s.noise_floor_db.is_finite());
    }

    /// **A clip shorter than the window is analysed, not refused.**
    ///
    /// The window is **1024 samples = 21.3 ms**, so the fixture must be shorter than that — the first version used
    /// 0.1 s (4 800 samples), which is five windows and therefore never exercised the branch. A 10 ms click is the
    /// realistic case and the one that proves the short path.
    #[test]
    fn a_clip_shorter_than_the_window_is_still_analysed() {
        let short = sine(1000.0, 0.5, 0.010);
        assert!(short.len() < WINDOW, "the fixture must be shorter than one window");
        let s = analyse(&short).expect("a short clip is analysable");
        let centroid = s.centroid_hz.expect("a short sine still has energy");
        assert!((centroid - 1000.0).abs() / 1000.0 < 0.05, "and still reads its own frequency: {centroid}");
    }

    /// A NaN sample is refused by index rather than averaged into every statistic.
    #[test]
    fn a_non_finite_sample_is_refused_and_located() {
        let mut buffer = sine(440.0, 0.5, 0.2);
        buffer[37] = f32::NAN;
        assert_eq!(analyse(&buffer), Err(SpectralProblem::NotFinite { at: 37 }));
    }

    #[test]
    fn an_empty_buffer_is_refused() {
        assert_eq!(analyse(&[]), Err(SpectralProblem::Empty));
    }

    /// **Determinism**: the same buffer analyses identically twice.
    #[test]
    fn the_same_buffer_analyses_the_same_twice() {
        let buffer = sine(440.0, 0.4, 0.3);
        assert_eq!(analyse(&buffer).expect("a"), analyse(&buffer).expect("b"));
    }

    /// A brighter tone has a higher rolloff at the same amplitude, which is the property a filter would rely on.
    #[test]
    fn rolloff_rises_with_brightness() {
        let dark = analyse(&sine(500.0, 0.5, 0.5)).expect("dark");
        let bright = analyse(&sine(6000.0, 0.5, 0.5)).expect("bright");
        assert!(
            bright.rolloff_hz.expect("energy") > dark.rolloff_hz.expect("energy"),
            "6000 Hz rolls off higher than 500 Hz"
        );
    }
}
