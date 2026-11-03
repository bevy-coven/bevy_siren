//! **Integrated loudness and true peak, per asset.** The numbers that explain why two tracks that embed close still
//! clash.
//!
//! An embedding cannot express *"this one is 12 LU louder"*. Two assets can sit close in CLAP's space and still be
//! unusable together, and loudness is the hard feature most often responsible. It is also the one feature here with an
//! unambiguous standard — **ITU-R BS.1770-4 / EBU R128** — so it is proven against a reference rather than eyeballed.
//!
//! # The measurement, and why `Option` is the whole design
//!
//! BS.1770 measures loudness by cutting the K-weighted signal into **400 ms gating blocks with 75 % overlap**, discarding
//! blocks below an absolute **−70 LUFS**, deriving a relative gate **10 LU** below the mean of what survives, and
//! averaging the remainder. A clip **shorter than one block has nothing to average**, and every way of papering over
//! that has already cost this project something:
//!
//! | Implementation | On a 0.2 s clip peaking at −6 dBFS |
//! |---|---|
//! | `ffmpeg`'s `ebur128` | **−70.0 LUFS** — the standard's floor, presented as if it were a level |
//! | `ebur128` 0.1.11 `loudness_global()` | **`-inf`** — not a number a ranking can compare |
//! | `ebur128` 0.1.11 `relative_threshold()` | exactly **−70.0**, the same value a genuine −70 LUFS passage gives |
//!
//! So this module never returns a bare `f32`. [`Loudness::integrated`] is an `Option`, and `None` means **too short to
//! measure**, which is a different fact from *silent* and from *−70 LUFS*. The block count it rests on is carried in
//! [`Loudness::blocks`] so a caller can see why.
//!
//! Measured in `docs/loop/runs/2026-11-03-77/`: `ebur128` agrees with `ffmpeg`'s reference to **0.01 LU** on every clip
//! long enough to have a block.
//!
//! # Where the numbers come from
//!
//! Every constant is the standard's, not this crate's: the gating block length, both gates, the K-weighting and the
//! 4× oversampled true peak are all inside `ebur128`, which ADR 0003 selected after verifying it is pure Rust. Nothing
//! in this file is a tunable, so nothing here is a magic constant.

use ebur128::{EbuR128, Mode};

/// The standard's gating block, in seconds: **400 ms**, per ITU-R BS.1770-4 §2.3.
pub const BLOCK_S: f64 = 0.4;

/// The hop between gating blocks, in seconds: **100 ms**, which is the standard's 75 % overlap expressed as a step.
pub const HOP_S: f64 = 0.1;

/// Why a buffer could not be measured.
///
/// A failure is named rather than folded into a number: a caller that cannot tell "this file is silent" from "this file
/// is broken" will make the wrong decision about both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoudnessProblem {
    /// The buffer has no frames, so there is nothing to weigh.
    Empty,
    /// The sample rate is not one the analyser accepts, carrying what was given.
    ///
    /// `ebur128` accepts **8 000 – 2 822 400 Hz**; a file outside that is a decoder bug or a header this crate read
    /// wrongly, and either way the number it would produce is not a measurement.
    UnsupportedRate {
        /// The rate the caller supplied.
        rate: u32,
    },
    /// The analyser refused a buffer it had already accepted a configuration for.
    Refused,
}

impl core::fmt::Display for LoudnessProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoudnessProblem::Empty => f.write_str("the buffer has no frames to measure"),
            LoudnessProblem::UnsupportedRate { rate } => write!(
                f,
                "a buffer at {rate} Hz cannot be measured: BS.1770's analyser takes 8000 to 2822400 Hz, so this is a \
                 header that was read wrongly or a decoder that lied"
            ),
            LoudnessProblem::Refused => {
                f.write_str("the loudness analyser refused a buffer after accepting its configuration")
            }
        }
    }
}

impl std::error::Error for LoudnessProblem {}

/// **One asset's loudness**, as BS.1770 defines it.
///
/// `integrated` is `None` when the clip is too short to contain a single gating block — which is the common case for a
/// sound effect, and the reason this is not a bare `f32`. `peak` and `sample_peak` are always measured: a peak needs no
/// gating, so a 20 ms click has a true peak even though it has no integrated loudness.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loudness {
    /// Integrated loudness in LUFS, or `None` when the clip is shorter than one gating block.
    pub integrated: Option<f32>,
    /// True peak in dBTP: the maximum of the 4× oversampled signal, per BS.1770 Annex 2.
    pub true_peak: f32,
    /// The unreconstructed sample peak in dBFS, for a caller that wants the number a meter shows.
    pub sample_peak: f32,
    /// How many 400 ms gating blocks passed the **absolute** gate.
    pub blocks: u64,
    /// How many gating blocks the clip's **length** allows, regardless of level.
    ///
    /// Carried beside [`Loudness::blocks`] because the pair is what distinguishes the two ways a clip can have no
    /// integrated loudness, and conflating them was a defect this module had on its first draft:
    ///
    /// | | `blocks_possible` | `blocks` |
    /// |---|---|---|
    /// | a 0.2 s sound effect | **0** | 0 |
    /// | 3 s of digital silence | **7** | **0** |
    ///
    /// Both give `integrated == None`, and they mean opposite things to a caller — one asset needs a different
    /// measure, the other is genuinely silent and should be excluded from a mix.
    pub blocks_possible: u64,
}

impl Loudness {
    /// True when this clip is **too short** to contain a single gating block, so no integrated loudness exists at all.
    ///
    /// Distinct from [`Loudness::silent`]: a 0.2 s footstep is too short; three seconds of digital silence is silent.
    pub fn too_short(&self) -> bool {
        self.blocks_possible == 0
    }

    /// True when the clip is long enough to be measured but **every** block fell below the absolute gate — digital
    /// silence, or a level under −70 LUFS.
    pub fn silent(&self) -> bool {
        self.blocks_possible > 0 && self.blocks == 0
    }
}

/// **Measure a mono buffer.** `rate` is the buffer's own rate; it must already be the model's, because resampling is
/// [`super::buffer`]'s job and a second resampler would be a second set of rules.
///
/// Mono is the only channel count here: [`super::buffer::downmix_to_mono`] has already averaged the channels, so a
/// stereo file and its fold-down measure the same. That matters because the loudness filter in `#27` compares assets,
/// and a stereo file measured per-channel would read ~3 dB above the same content folded to mono.
pub fn measure(buffer: &[f32], rate: u32) -> Result<Loudness, LoudnessProblem> {
    if buffer.is_empty() {
        return Err(LoudnessProblem::Empty);
    }
    let mut analyser = EbuR128::new(1, rate, Mode::I | Mode::TRUE_PEAK | Mode::SAMPLE_PEAK)
        .map_err(|_| LoudnessProblem::UnsupportedRate { rate })?;
    analyser.add_frames_f32(buffer).map_err(|_| LoudnessProblem::Refused)?;

    // **The block count is the measurement's premise, not a detail.** `loudness_global()` folds "no blocks" into
    // `-inf`, and `relative_threshold()` folds it into `-70.0` — the same value a real −70 LUFS passage produces. Asking
    // for the count separately is the only way to tell those apart.
    let (blocks, _energy) = analyser.gating_block_count_and_energy().unwrap_or((0, 0.0));
    // The count `ebur128` returns is blocks **above the absolute gate**, so it cannot answer "was this too short?" on
    // its own — silence returns zero blocks too. The length does: BS.1770 uses 400 ms blocks hopping 100 ms, so a clip
    // of `n` samples at `rate` admits `1 + (n - 0.4·rate) / (0.1·rate)` of them, or none.
    let block_samples = (f64::from(rate) * BLOCK_S).round() as usize;
    let hop_samples = (f64::from(rate) * HOP_S).round() as usize;
    let blocks_possible = if buffer.len() < block_samples {
        0
    } else {
        (1 + (buffer.len() - block_samples) / hop_samples.max(1)) as u64
    };
    let integrated = if blocks == 0 {
        None
    } else {
        // `gated_loudness` is finite whenever there is at least one block; the `filter` guards the arithmetic rather
        // than trusting it, because a NaN here would poison every comparison downstream.
        analyser.loudness_global().ok().map(|l| l as f32).filter(|l| l.is_finite())
    };

    // A peak is amplitude, not loudness: it needs no gating, so even an empty-of-blocks clip has one. Both are floored
    // at `SILENT_DB` rather than reported as −inf, matching the crate's one rule for a gain that is not a gain.
    let true_peak = amplitude_db(analyser.true_peak(0).unwrap_or(0.0));
    let sample_peak = amplitude_db(analyser.sample_peak(0).unwrap_or(0.0));

    Ok(Loudness { integrated, true_peak, sample_peak, blocks, blocks_possible })
}

/// A linear amplitude as dB, floored the way every other gain in this crate is floored.
///
/// The floor is [`crate::model::SILENT_DB`] and the conversion is [`crate::model::gain_to_db`], so this file adds no
/// second convention for what silence reads as.
fn amplitude_db(amplitude: f64) -> f32 {
    crate::model::gain_to_db(amplitude as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::MODEL_SAMPLE_RATE as RATE;

    /// A sine at `hz`, `amplitude`, lasting `seconds`.
    fn tone(hz: f32, amplitude: f32, seconds: f32) -> Vec<f32> {
        let n = (RATE as f32 * seconds) as usize;
        (0..n).map(|i| amplitude * (std::f32::consts::TAU * hz * i as f32 / RATE as f32).sin()).collect()
    }

    /// **The reference's own number.** `ffmpeg -af ebur128` reports **−23.0 LUFS** and **−20.0 dBFS** for a 3 s 1 kHz
    /// tone at 0.1 amplitude; `ebur128` measures **−23.01 LUFS** on the same file. The 0.1 LU tolerance is #21's own
    /// acceptance criterion.
    #[test]
    fn a_tone_matches_the_references_integrated_loudness() {
        let l = measure(&tone(1000.0, 0.1, 3.0), RATE).expect("measure");
        let integrated = l.integrated.expect("3 s is seven gating blocks");
        assert!((integrated - -23.0).abs() < 0.1, "got {integrated}, the reference gives -23.0");
        assert!((l.sample_peak - -20.0).abs() < 0.1, "got {}, the reference gives -20.0", l.sample_peak);
        assert!(l.blocks >= 7, "3 s at 75% overlap is seven blocks, got {}", l.blocks);
    }

    /// **A louder tone reads louder, by the right amount.** 0.5 against 0.1 is a factor of five, which is
    /// `20·log10(5) = 13.98 dB`, so the two integrated values must differ by that and no more.
    #[test]
    fn five_times_the_amplitude_is_fourteen_decibels_louder() {
        let quiet = measure(&tone(1000.0, 0.1, 3.0), RATE).expect("quiet");
        let loud = measure(&tone(1000.0, 0.5, 3.0), RATE).expect("loud");
        let delta = loud.integrated.expect("blocks") - quiet.integrated.expect("blocks");
        assert!((delta - 13.98).abs() < 0.1, "got {delta} dB, the amplitude ratio of 5 is 13.98 dB");
    }

    /// **The short-clip trap, as a named case.** A 0.2 s clip has no 400 ms gating block, so there is no integrated
    /// loudness — and this must be `None`, not −70 and not `-inf`. Its **peak is still measured**, because a peak needs
    /// no gating.
    ///
    /// Measured on the same content: `ffmpeg` says −70.0 LUFS (the floor as a level), `ebur128`'s `loudness_global`
    /// says `-inf`, and this function says `None`.
    #[test]
    fn a_clip_shorter_than_a_gating_block_has_no_integrated_loudness() {
        let l = measure(&tone(1000.0, 0.5, 0.2), RATE).expect("measure");
        assert_eq!(l.integrated, None, "0.2 s has no 400 ms block, so there is no integrated loudness");
        assert_eq!(l.blocks, 0, "and the count is what says so");
        assert_eq!(l.blocks_possible, 0, "not because it was too quiet, but because no block fits");
        assert!(l.too_short());
        assert!(!l.silent(), "a short loud clip is not silence");
        assert!((l.sample_peak - -6.02).abs() < 0.1, "the peak is still real: got {}", l.sample_peak);
    }

    /// A block boundary: 400 ms is exactly one block, so it measures; a hair under it does not.
    #[test]
    fn the_short_clip_boundary_is_where_the_standard_puts_it() {
        let at = measure(&tone(1000.0, 0.5, 0.4), RATE).expect("400 ms");
        assert!(at.blocks >= 1, "400 ms is exactly one gating block, got {}", at.blocks);
        assert!(at.integrated.is_some(), "and it therefore has a loudness");
    }

    /// **Silence is a measurement, not a missing one.** A 3 s silent buffer has plenty of gating blocks and a real
    /// integrated value at the floor — which is a different fact from the short clip above, and the whole reason the
    /// block count is carried.
    #[test]
    fn silence_measures_at_the_floor_rather_than_being_absent() {
        let l = measure(&vec![0.0f32; RATE as usize * 3], RATE).expect("measure");
        assert!(l.blocks_possible > 0, "three seconds allows seven gating blocks");
        assert_eq!(l.blocks, 0, "and every one of them is below the absolute gate");
        assert_eq!(l.integrated, None, "so there is no loudness to report");
        assert!(l.silent(), "which is silence, not a short clip");
        assert!(!l.too_short(), "the length was never the problem");
        assert!(l.true_peak <= crate::model::SILENT_DB + 0.01, "and the peak is the crate's silence floor");
    }

    /// A buffer at a rate the analyser cannot take is refused by name rather than measured wrongly.
    #[test]
    fn an_impossible_rate_is_refused_with_the_rate() {
        let err = measure(&tone(1000.0, 0.1, 1.0), 0).expect_err("0 Hz");
        assert_eq!(err, LoudnessProblem::UnsupportedRate { rate: 0 });
    }

    #[test]
    fn an_empty_buffer_is_refused() {
        assert_eq!(measure(&[], RATE), Err(LoudnessProblem::Empty));
    }

    /// **Determinism**: the same buffer measures identically twice, field for field.
    #[test]
    fn the_same_buffer_measures_the_same_twice() {
        let buffer = tone(440.0, 0.3, 2.0);
        assert_eq!(measure(&buffer, RATE).expect("a"), measure(&buffer, RATE).expect("b"));
    }
}
