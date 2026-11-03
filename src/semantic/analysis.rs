//! **One pass over the frames, for everything that reads a spectrum.**
//!
//! Three features arrived in three iterations and each walked the audio itself:
//!
//! | feature | iteration | what it computed per frame |
//! |---|---|---|
//! | spectral | 89 | a 1024-point FFT, power summed by bin |
//! | tempo | 91 | a 1024-point FFT, **energy only** |
//! | key | 92 | an **8192-point** FFT, band-integrated into a chroma |
//!
//! Spectral and tempo use the *same* window and the *same* hop, so they were computing the same transform twice. Measured
//! on a 4 s click track: loudness **2.40 ms**, spectral **3.96 ms**, tempo **4.23 ms**, key **4.69 ms** — 15.3 ms per
//! file, of which ~4 ms was duplicated work.
//!
//! This module is that work done once. It is **not** a general framework: it produces exactly the two things the callers
//! need — the power spectrum summed across frames, and the per-frame energy — and key keeps its own transform because a
//! chroma needs 8192 points and this pass needs 1024.
//!
//! # What it does not change
//!
//! The numbers. `spectral` and `tempo` are unchanged in behaviour, and their tests are the proof: the same frame starts,
//! the same window, the same `|z|²`. The only difference is that the FFT is evaluated once per frame instead of twice.

use super::mel::{BINS, HOP, MEL_FLOOR, WINDOW, fft_in_place, hann};

/// **The spectra of every frame, from one transform per frame.**
pub struct Spectra {
    /// Power summed across every frame, per bin. What a centroid or a rolloff is computed from.
    pub summed_power: Vec<f64>,
    /// The total power of each frame, in order — what an onset envelope is computed from.
    pub frame_energy: Vec<f32>,
    /// How many samples each frame's window covered, which is shorter than [`WINDOW`] only for a clip shorter than one.
    pub frame_len: usize,
}

/// **Analyse a buffer's frames once**, for every consumer that needs a 1024-point spectrum.
///
/// The frame starts and the padding behaviour are the ones `spectral` and `tempo` already agreed on: a clip shorter than
/// one window is analysed as a **single frame of its own length**, not zero-padded, because padding a 0.02 s click to
/// 1024 samples would dilute every statistic with silence.
///
/// A buffer holding a non-finite sample is refused by the callers before this runs — they each check, and this pass
/// assumes they did, so an empty buffer is the only thing it guards.
pub fn spectra(buffer: &[f32]) -> Spectra {
    let window = hann(WINDOW);
    let frame_len = buffer.len().min(WINDOW);
    let starts: Vec<usize> =
        if buffer.len() <= WINDOW { vec![0] } else { (0..=buffer.len() - WINDOW).step_by(HOP).collect() };

    let mut summed_power = vec![0.0f64; BINS];
    let mut frame_energy = Vec::with_capacity(starts.len());
    // **Allocated once, reused per frame.** The first version of tempo's own helper allocated two `WINDOW`-sized
    // vectors inside the loop, which for a 4 s clip is 400 allocations of 16 KB — the kind of cost that hides behind a
    // correct answer.
    let mut re = vec![0.0f64; WINDOW];
    let mut im = vec![0.0f64; WINDOW];

    for &start in &starts {
        for i in 0..WINDOW {
            let sample = if i < frame_len { buffer[start + i.min(buffer.len() - 1 - start)] } else { 0.0 };
            re[i] = f64::from(sample) * f64::from(window[i]);
            im[i] = 0.0;
        }
        fft_in_place(&mut re, &mut im);
        let mut total = 0.0f64;
        for (bin, slot) in summed_power.iter_mut().enumerate() {
            let p = re[bin] * re[bin] + im[bin] * im[bin];
            *slot += p;
            total += p;
        }
        frame_energy.push(total.max(f64::from(MEL_FLOOR)) as f32);
    }
    Spectra { summed_power, frame_energy, frame_len }
}

/// **The onset-strength envelope** from a frame-energy sequence: the positive frame-to-frame rise.
///
/// An onset is a **rise**; a decay is not a second onset, which is why only the positive part is kept.
pub fn onset_envelope(frame_energy: &[f32]) -> Vec<f32> {
    frame_energy.windows(2).map(|w| (w[1] - w[0]).max(0.0)).collect()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn sine(hz: f32, seconds: f32) -> Vec<f32> {
        let n = (super::super::MODEL_SAMPLE_RATE as f32 * seconds) as usize;
        (0..n)
            .map(|i| {
                0.5 * (std::f32::consts::TAU * hz * i as f32 / super::super::MODEL_SAMPLE_RATE as f32).sin()
            })
            .collect()
    }

    /// **The frame count is the one both callers already used**: `1 + (len − WINDOW) / HOP` windows, or one frame when
    /// the clip is shorter than a window.
    #[test]
    fn the_frame_count_matches_the_callers() {
        let s = spectra(&sine(440.0, 1.0));
        assert_eq!(s.frame_energy.len(), 1 + (48_000 - WINDOW) / HOP);
        // And a clip shorter than a window is ONE frame of its own length.
        let short = spectra(&sine(440.0, 0.010));
        assert_eq!(short.frame_energy.len(), 1);
        assert_eq!(short.frame_len, 480);
    }

    /// **A pure tone's power sits in the bins around its frequency**, which is the property both consumers rely on.
    #[test]
    fn a_tones_power_lands_in_its_own_bins() {
        let s = spectra(&sine(1000.0, 1.0));
        let bin_hz = 48_000.0 / WINDOW as f64;
        let expected = (1000.0 / bin_hz).round() as usize;
        // The strongest bin is the one nearest the tone, or its neighbour, given Hann leakage.
        let peak = s
            .summed_power
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).expect("no NaN"))
            .expect("a spectrum")
            .0;
        assert!(peak.abs_diff(expected) <= 1, "peak bin {peak} against {expected} for a 1 kHz tone");
    }

    /// **A silent buffer gives zero power everywhere**, not NaN and not a panic.
    #[test]
    fn silence_gives_zero_power() {
        let s = spectra(&vec![0.0f32; 4096]);
        assert!(s.summed_power.iter().all(|p| *p == 0.0));
        assert!(s.frame_energy.iter().all(|e| e.is_finite()));
    }

    /// **An onset envelope is the positive rise**, so a decay contributes nothing.
    #[test]
    fn an_onset_envelope_keeps_only_the_rises() {
        assert_eq!(onset_envelope(&[1.0, 3.0, 2.0, 5.0]), vec![2.0, 0.0, 3.0]);
        assert!(onset_envelope(&[4.0, 3.0, 2.0, 1.0]).iter().all(|v| *v == 0.0), "a decay is not an onset");
    }

    /// Determinism: the same buffer gives the same spectra twice.
    #[test]
    fn the_same_buffer_gives_the_same_spectra() {
        let b = sine(440.0, 0.5);
        assert_eq!(spectra(&b).summed_power, spectra(&b).summed_power);
        assert_eq!(spectra(&b).frame_energy, spectra(&b).frame_energy);
    }
}
