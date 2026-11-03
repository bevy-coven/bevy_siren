//! **A file, decoded to the buffer a model can consume.**
//!
//! `buffer.rs` owns the arithmetic — downmix, rate agreement — and this module owns the one thing that needs a
//! decoder: turning a path into interleaved samples plus the rate and channel count the header promised.
//!
//! # One decoder, and it is one the crate already has
//!
//! `rodio` is already a dependency behind `mixer`, and it decodes wav, vorbis and flac through the features this crate
//! enables. The semantic feature therefore **does not bring a second decoder**: an asset indexed by `semantic` and an
//! asset played by `mixer` are decoded by the same code, so a file that plays is a file that indexes.
//!
//! That is also why `semantic` names `rodio`: **it costs no new crate.** `bevy_audio` already resolves `rodio 0.22.2`
//! in the default graph — measured: `cargo tree -i rodio` reports `bevy_audio → bevy_internal → bevy → bevy_siren` —
//! so enabling the feature's own `rodio` entry turns on the decoder features this crate declares and adds nothing to
//! the graph. The default build stays **413 lines**, unchanged.
//!
//! # The rate
//!
//! A file at the model's rate is downmixed and returned; a file at any other rate is downmixed and **resampled** by
//! [`super::resample`], which is the commonest case in a game's library. `buffer::model_buffer` still refuses a rate it
//! cannot convert, and that remains the low-level contract — but nothing on this path reaches it with a rate other than
//! the model's, because the conversion happens first.

use std::fs::File;
use std::path::Path;

use rodio::Source;

use super::buffer::{BufferProblem, downmix_to_mono};
use super::resample;

/// Why a file could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeProblem {
    /// The file could not be opened.
    Open {
        /// The path, so the message points at a file.
        path: String,
        /// The operating system's message, passed through rather than paraphrased.
        error: String,
    },
    /// The file opened but its bytes are not audio this crate decodes.
    Undecodable {
        /// The path.
        path: String,
        /// What the decoder said.
        error: String,
    },
    /// It decoded, but the result cannot be prepared for the model.
    Buffer(BufferProblem),
}

impl core::fmt::Display for DecodeProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeProblem::Open { path, error } => write!(f, "cannot open `{path}`: {error}"),
            DecodeProblem::Undecodable { path, error } => {
                write!(f, "`{path}` is not audio this crate decodes: {error}")
            }
            DecodeProblem::Buffer(b) => write!(f, "{b}"),
        }
    }
}

/// What a decoded file gives: the samples, and what the decoder said about them.
///
/// Kept together rather than returning a bare `Vec<f32>`, because the rate is *why* the buffer may be refused and a
/// caller that dropped it could not report the reason.
#[derive(Debug, Clone, PartialEq)]
pub struct Decoded {
    /// Interleaved samples, `channels` values per frame.
    pub samples: Vec<f32>,
    /// The file's own sample rate, before any conversion.
    pub sample_rate: u32,
    /// The file's channel count.
    pub channels: u16,
}

impl Decoded {
    /// The number of frames, which is not the number of samples when a file is not mono.
    pub fn frames(&self) -> usize {
        if self.channels == 0 { 0 } else { self.samples.len() / self.channels as usize }
    }

    /// The duration in seconds, as the file's own rate defines it.
    pub fn seconds(&self) -> f64 {
        if self.sample_rate == 0 { 0.0 } else { self.frames() as f64 / f64::from(self.sample_rate) }
    }
}

/// **Decode a file** to interleaved samples, with the rate and channels the stream reports.
///
/// The rate is read from the decoded stream rather than from `library`'s header parse, so the two are independent
/// confirmations of the same file rather than one trusting the other.
pub fn decode(path: &Path) -> Result<Decoded, DecodeProblem> {
    let shown = path.display().to_string();
    let file =
        File::open(path).map_err(|e| DecodeProblem::Open { path: shown.clone(), error: e.to_string() })?;
    let decoder = rodio::Decoder::try_from(file)
        .map_err(|e| DecodeProblem::Undecodable { path: shown.clone(), error: e.to_string() })?;

    // `rodio` reports these as `NonZero`, which is a stronger promise than a `u32` check: a stream cannot declare
    // 0 Hz or 0 channels, so the two `BufferProblem` variants for those become unreachable through this path.
    let sample_rate = decoder.sample_rate().get();
    let channels = decoder.channels().get();
    let samples: Vec<f32> = decoder.collect();
    Ok(Decoded { samples, sample_rate, channels })
}

/// **Decode and prepare in one step**: the whole path from a file to a model input, or the reason it stops.
///
/// One function so there is one path, per the crate's rule that a second path is a second set of rules.
///
/// A file at the model's rate is downmixed and returned. **A file at any other rate is downmixed and resampled**, which
/// is the commonest case in a game's library — 44.1 kHz is what asset stores ship. The conversion is `rubato`'s FFT
/// resampler, chosen by measurement: it keeps a 1 kHz tone at **0.5000 of 0.5000** of its energy and yields **exactly
/// 48000** samples from a 1.000 s input (`runs/2026-11-03-65/research.md`).
///
/// A rate of zero is still refused, because nothing can be divided by it.
pub fn decode_for_model(path: &Path) -> Result<Vec<f32>, DecodeProblem> {
    let decoded = decode(path)?;
    let mono = downmix_to_mono(&decoded.samples, decoded.channels).map_err(DecodeProblem::Buffer)?;
    if mono.is_empty() {
        return Err(DecodeProblem::Buffer(BufferProblem::Empty));
    }
    resample::to_model_rate(&mono, decoded.sample_rate).map_err(DecodeProblem::Buffer)
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`DecodeProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for DecodeProblem {}
