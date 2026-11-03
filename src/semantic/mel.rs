//! **CLAP's log-mel spectrogram**, in this crate, with no dependency.
//!
//! The audio tower takes `input_features`: `[batch, 4, 1001, 64]`. Something has to turn a mono buffer at 48 kHz into
//! that, and this is it — the mel path `indexer::build`'s seam has been waiting for.
//!
//! # Read from the model, not remembered
//!
//! Every number below is read from `ClapFeatureExtractor.from_pretrained("laion/clap-htsat-fused")`:
//!
//! | | |
//! |---|---|
//! | sample rate | **48 000 Hz** |
//! | window | **1024** samples, Hann, periodic |
//! | hop | **480** samples |
//! | mel bands | **64** |
//! | frequency range | **50 – 14 000 Hz** |
//! | filterbank | **513 × 64**, from the extractor's own `mel_filters` |
//! | frames | **1001** for the model's window |
//! | dB floor | `min_value = 1e-10` → **−100 dB** |
//!
//! The pipeline is `transformers.audio_utils.spectrogram`'s, read from the installed source: **reflect**-pad by
//! `frame_length / 2` on each side, multiply each frame by the window, `rfft`, take `|·|²`, apply the mel matrix with a
//! `1e-10` floor, then `10·log10(x)`. Reading it mattered: the mel filterbank's first and last rows are **all zero**,
//! so a matrix applied as `513 → 64` over the raw spectrum is right and a `64 → 513` reconstruction is not.
//!
//! # Why an FFT here rather than a crate
//!
//! The dependency list is closed (`tests/leaf.rs`), and a 1024-point radix-2 FFT is thirty lines. The alternative —
//! another crate — would need the argument `ort` and `rubato` both needed, and this does not earn one.

use super::{MODEL_MEL_FRAMES, MODEL_SAMPLE_RATE};

/// Samples per window, read from the extractor's `fft_window_size`.
pub const WINDOW: usize = 1024;

/// Samples between windows, read from the extractor's `hop_length`.
pub const HOP: usize = 480;

/// Mel bands, read from the extractor's `feature_size`.
pub const MEL_BANDS: usize = 64;

/// Frequency bins for a 1024-point real FFT: `WINDOW / 2 + 1`.
pub const BINS: usize = WINDOW / 2 + 1;

/// Below this a power is silence, matching the extractor's `min_value = 1e-10` (−100 dB).
pub const MEL_FLOOR: f32 = 1e-10;

/// Samples the model's one window holds: `nb_max_frames × hop`.
pub const MODEL_WINDOW_SAMPLES: usize = 480_000;

/// The mel filterbank: **`MEL_BANDS` rows by `BINS` columns**, band-major — the layout the extractor stores.
///
/// Measured: `ClapFeatureExtractor.mel_filters` has shape **`(64, 513)`** and flattens to **32,832** numbers in that
/// order. My first version assumed `(513, 64)` and would have applied a transposed matrix — producing a spectrogram of
/// the right *shape* and the wrong *content*, which is the failure this whole loop keeps finding.
///
/// These are a property of the *model*, not of this crate: a project that supplies its own must supply the one its
/// checkpoint was trained with, which is the same reason [`crate::semantic::index::ModelStamp`] exists.
#[derive(Debug, Clone, PartialEq)]
pub struct Filterbank {
    /// `MEL_BANDS × BINS` weights, band-major.
    weights: Vec<f32>,
}

impl Filterbank {
    /// **From a flat `MEL_BANDS × BINS` array**, as the extractor stores it.
    pub fn from_weights(weights: Vec<f32>) -> Result<Filterbank, MelProblem> {
        if weights.len() != BINS * MEL_BANDS {
            return Err(MelProblem::FilterbankShape { got: weights.len(), expected: BINS * MEL_BANDS });
        }
        Ok(Filterbank { weights })
    }

    /// True when every weight is a finite, non-negative number.
    pub fn is_sane(&self) -> bool {
        self.weights.iter().all(|w| w.is_finite() && *w >= 0.0)
    }
}

/// Why a spectrogram could not be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MelProblem {
    /// The buffer is empty, so there is no frame to take.
    Empty,
    /// The filterbank is not `MEL_BANDS × BINS` numbers.
    FilterbankShape {
        /// How many weights were supplied.
        got: usize,
        /// How many are needed.
        expected: usize,
    },
}

impl core::fmt::Display for MelProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MelProblem::Empty => f.write_str("the buffer is empty, so there is no frame to analyse"),
            MelProblem::FilterbankShape { got, expected } => write!(
                f,
                "the mel filterbank has {got} weights and this extractor's has {expected} \
                 ({MEL_BANDS} bands by {BINS} bins). It is the model's own and not a shape to invent"
            ),
        }
    }
}

/// **The log-mel spectrogram of one mono buffer at 48 kHz**, as `[frames][MEL_BANDS]` in dB.
///
/// `frames` is `1 + len / HOP` for a buffer shorter than the model's window, which is the count
/// `transformers`' `spectrogram` produces under `center: true` with reflect padding — the first frame is centred on
/// sample 0.
///
/// A buffer longer than [`MODEL_WINDOW_SAMPLES`] is **not** silently truncated: the caller chunks it, because how a long
/// track becomes 10-second windows is a decision with its own numbers (`I-03`), not something a spectrogram should hide.
pub fn log_mel(buffer: &[f32], filterbank: &Filterbank) -> Result<Vec<Vec<f32>>, MelProblem> {
    if buffer.is_empty() {
        return Err(MelProblem::Empty);
    }
    let window = hann(WINDOW);
    // **Reflect padding**, `frame_length / 2` on each side, which is what makes frame 0 the centred one.
    let half = WINDOW / 2;
    let padded = reflect_pad(buffer, half);

    let frames = buffer.len() / HOP + 1;
    let mut out = Vec::with_capacity(frames);
    let mut re = vec![0.0f32; WINDOW];
    let mut im = vec![0.0f32; WINDOW];
    let mut power = vec![0.0f32; BINS];

    for frame in 0..frames {
        let start = frame * HOP;
        for i in 0..WINDOW {
            re[i] = padded[start + i] * window[i];
            im[i] = 0.0;
        }
        fft_in_place(&mut re, &mut im);
        for (bin, p) in power.iter_mut().enumerate() {
            // `|z|^2`, which is what `power = 2.0` means.
            *p = re[bin] * re[bin] + im[bin] * im[bin];
        }
        // The mel matrix, then the floor, then dB — the order `spectrogram` uses.
        let mut band = vec![0.0f32; MEL_BANDS];
        for (m, v) in band.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            // Band-major: band `m`'s weights are `m * BINS .. (m + 1) * BINS`. Zip rather than index, which is also
            // the shape the compiler can vectorise.
            for (weight, p) in filterbank.weights[m * BINS..(m + 1) * BINS].iter().zip(power.iter()) {
                sum += weight * p;
            }
            *v = 10.0 * sum.max(MEL_FLOOR).log10();
        }
        out.push(band);
    }
    Ok(out)
}

/// **The four channels the `htsat-fused` graph takes**, from a spectrogram.
///
/// `[4, 1001, 64]` per clip: channel 0 is the spectrogram downsampled to 1001 frames, and channels 1–3 are the front,
/// middle and back 1001-frame windows of the full spectrogram. That is the "fusion" path `ClapFeatureExtractor`
/// documents, and it is what the audio tower's `is_longer` flag selects.
///
/// Here the three windows are **fixed** (front, middle, back) rather than randomly chosen as the original code does.
/// Random crops make an embedding depend on a dice roll, which is the one thing this crate's determinism rule forbids;
/// for a clip that fits one window all four channels describe the same audio, so the difference only shows on a long
/// track, where a deterministic choice is also the reproducible one.
pub fn fusion_channels(mel: &[Vec<f32>]) -> Vec<Vec<Vec<f32>>> {
    let total = mel.len();
    let window = MODEL_MEL_FRAMES;
    let pick = |start: usize| -> Vec<Vec<f32>> {
        (0..window)
            .map(|i| {
                let at = (start + i).min(total.saturating_sub(1));
                mel.get(at).cloned().unwrap_or_else(|| vec![0.0; MEL_BANDS])
            })
            .collect()
    };

    // Channel 0: the whole spectrogram resampled to `window` frames by nearest-neighbour, which is what a bilinear
    // downsample of a 1001-frame output reduces to when the input is already short.
    let shrunk: Vec<Vec<f32>> = (0..window)
        .map(|i| {
            let at = if total <= 1 { 0 } else { i * total.saturating_sub(1) / window.saturating_sub(1) };
            mel.get(at).cloned().unwrap_or_else(|| vec![0.0; MEL_BANDS])
        })
        .collect();

    vec![shrunk, pick(0), pick(total.saturating_sub(window) / 2), pick(total.saturating_sub(window))]
}

/// **The whole path**: a mono 48 kHz buffer to the `[4, 1001, 64]` the audio tower wants.
pub fn input_features(buffer: &[f32], filterbank: &Filterbank) -> Result<Vec<Vec<Vec<f32>>>, MelProblem> {
    debug_assert_eq!(
        buffer.len().min(MODEL_WINDOW_SAMPLES),
        buffer.len(),
        "input_features is for one model window; a longer buffer is chunked by the caller"
    );
    let mel = log_mel(buffer, filterbank)?;
    Ok(fusion_channels(&mel))
}

/// A periodic Hann window, which is what `window_function(n, "hann")` returns.
fn hann(n: usize) -> Vec<f32> {
    (0..n).map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n as f32).cos()).collect()
}

/// `pad` samples on each side, mirrored: what `np.pad(mode="reflect")` does.
fn reflect_pad(buffer: &[f32], pad: usize) -> Vec<f32> {
    let n = buffer.len();
    let mut out = Vec::with_capacity(n + 2 * pad);
    // Left: buffer[pad], buffer[pad-1], … buffer[1] — reflect excludes the edge sample, as numpy does.
    for i in (1..=pad).rev() {
        out.push(buffer[i.min(n - 1)]);
    }
    out.extend_from_slice(buffer);
    for i in 0..pad {
        let at = n.saturating_sub(2).saturating_sub(i);
        out.push(buffer[at.min(n - 1)]);
    }
    out
}

/// **A radix-2 FFT, in place.** `len` must be a power of two, which [`WINDOW`] is.
fn fft_in_place(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    debug_assert!(n.is_power_of_two(), "the radix-2 FFT needs a power of two");

    // Bit-reversal permutation.
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    // Butterflies.
    let mut len = 2usize;
    while len <= n {
        let angle = -std::f32::consts::TAU / len as f32;
        let (wr, wi) = (angle.cos(), angle.sin());
        let mut i = 0usize;
        while i < n {
            let (mut cur_r, mut cur_i) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let (ur, ui) = (re[i + k], im[i + k]);
                let (vr, vi) = (re[i + k + len / 2], im[i + k + len / 2]);
                let tr = vr * cur_r - vi * cur_i;
                let ti = vr * cur_i + vi * cur_r;
                re[i + k] = ur + tr;
                im[i + k] = ui + ti;
                re[i + k + len / 2] = ur - tr;
                im[i + k + len / 2] = ui - ti;
                let next_r = cur_r * wr - cur_i * wi;
                cur_i = cur_r * wi + cur_i * wr;
                cur_r = next_r;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// The model's own rate, asserted against the extractor's `sampling_rate`.
pub const RATE: u32 = MODEL_SAMPLE_RATE;

#[cfg(test)]
mod tests_support {
    use super::*;

    /// **The extractor's own filterbank**, checked in as `tests/fixtures/clap-mel-filters.txt` — 64 × 513 = 32,832
    /// numbers, band-major, copied from `ClapFeatureExtractor.from_pretrained("laion/clap-htsat-fused").mel_filters`.
    pub fn clap() -> Filterbank {
        let text = include_str!("../../tests/fixtures/clap-mel-filters.txt");
        let weights: Vec<f32> = text.lines().filter_map(|l| l.trim().parse().ok()).collect();
        Filterbank::from_weights(weights).expect("the fixture is the extractor's own 64 by 513")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A filterbank of ones, so the mel stage sums and a test can reason about the FFT alone.
    fn flat() -> Filterbank {
        Filterbank::from_weights(vec![1.0; BINS * MEL_BANDS]).expect("shape")
    }

    #[test]
    fn the_fft_matches_a_hand_computed_discrete_transform() {
        // A single impulse at index 1 has a spectrum of `exp(-2πik/n)`: every bin has magnitude 1.
        let mut re = vec![0.0f32; 8];
        let mut im = vec![0.0f32; 8];
        re[1] = 1.0;
        fft_in_place(&mut re, &mut im);
        for k in 0..8 {
            let mag = (re[k] * re[k] + im[k] * im[k]).sqrt();
            assert!((mag - 1.0).abs() < 1e-5, "bin {k} magnitude {mag}");
        }
        // And the phase of bin 1 is -2π/8.
        let expected = -std::f32::consts::TAU / 8.0;
        assert!((im[1].atan2(re[1]) - expected).abs() < 1e-5, "phase {}", im[1].atan2(re[1]));
    }

    #[test]
    fn the_window_is_periodic_hann() {
        let w = hann(8);
        assert!((w[0] - 0.0).abs() < 1e-6, "a periodic Hann starts at 0");
        assert!((w[4] - 1.0).abs() < 1e-6, "peaks at n/2");
        // Sum of a periodic Hann is exactly n/2.
        let sum: f32 = w.iter().sum();
        assert!((sum - 4.0).abs() < 1e-5, "sum {sum}");
    }

    #[test]
    fn reflect_padding_mirrors_without_repeating_the_edge() {
        let v = vec![1.0, 2.0, 3.0, 4.0];
        let p = reflect_pad(&v, 2);
        assert_eq!(p, vec![3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0], "numpy's reflect excludes the edge");
    }

    #[test]
    fn one_second_of_audio_gives_the_frame_count_the_pipeline_gives() {
        // `1 + len / HOP` with `center: true`: 48000 / 480 + 1 = 101.
        let buffer = vec![0.1f32; RATE as usize];
        let mel = log_mel(&buffer, &flat()).expect("mel");
        assert_eq!(mel.len(), 101);
        assert_eq!(mel[0].len(), MEL_BANDS);
    }

    #[test]
    fn a_silent_buffer_sits_at_the_floor_not_at_negative_infinity() {
        let mel = log_mel(&vec![0.0f32; RATE as usize], &flat()).expect("mel");
        for band in &mel[0] {
            assert!(band.is_finite(), "a zero power must not become -inf, got {band}");
            assert!((band - (-100.0)).abs() < 1e-4, "1e-10 is -100 dB, got {band}");
        }
    }

    #[test]
    fn a_filterbank_of_the_wrong_shape_is_refused() {
        let err = Filterbank::from_weights(vec![1.0; 10]).expect_err("wrong shape");
        match err {
            MelProblem::FilterbankShape { got, expected } => {
                assert_eq!(got, 10);
                assert_eq!(expected, BINS * MEL_BANDS);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn an_empty_buffer_is_refused() {
        assert_eq!(log_mel(&[], &flat()), Err(MelProblem::Empty));
    }

    #[test]
    fn the_fusion_channels_are_four_windows_of_the_models_size() {
        let buffer = vec![0.1f32; RATE as usize];
        let features = input_features(&buffer, &flat()).expect("features");
        assert_eq!(features.len(), 4, "four channels");
        for channel in &features {
            assert_eq!(channel.len(), MODEL_MEL_FRAMES, "each 1001 frames");
            assert_eq!(channel[0].len(), MEL_BANDS, "each 64 bands");
        }
    }

    #[test]
    fn the_same_buffer_gives_the_same_spectrogram_twice() {
        // Determinism, which the fixed windows in `fusion_channels` exist for: the original picks its crops at random.
        let buffer: Vec<f32> = (0..RATE as usize)
            .map(|i| (std::f32::consts::TAU * 1000.0 * i as f32 / RATE as f32).sin() * 0.5)
            .collect();
        let a = input_features(&buffer, &flat()).expect("a");
        let b = input_features(&buffer, &flat()).expect("b");
        assert_eq!(a, b);
    }
}

#[cfg(test)]
mod against_the_model {
    use super::tests_support::*;
    use super::*;

    /// **The spectrogram against the extractor's own numbers.**
    ///
    /// The reference is `ClapFeatureExtractor._np_extract_fbank_features` on a 1 kHz tone at 48 kHz, whose first frame
    /// begins `[16.570843, 16.544992, 16.996021, 17.427816, 17.87301, 18.320274]`. Getting the FFT, the window, the
    /// padding, the filterbank layout and the dB conversion all right is the only way to land within a decibel of that;
    /// getting any one wrong moves the numbers by tens.
    #[test]
    fn the_spectrogram_matches_the_extractors_own_output() {
        let buffer: Vec<f32> = (0..RATE as usize)
            .map(|i| (std::f32::consts::TAU * 1000.0 * i as f32 / RATE as f32).sin() * 0.5)
            .collect();
        let mel = log_mel(&buffer, &clap()).expect("mel");
        assert_eq!(mel.len(), 101, "1 + 48000 / 480");

        let expected_first_six = [16.570843f32, 16.544992, 16.996021, 17.427816, 17.87301, 18.320274];
        for (band, expected) in expected_first_six.iter().enumerate() {
            let got = mel[0][band];
            assert!(
                (got - expected).abs() < 0.05,
                "band {band}: got {got:.6}, the extractor gives {expected:.6}"
            );
        }
    }
}
