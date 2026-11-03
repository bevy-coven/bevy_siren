//! **The buffer a model consumes, and the arithmetic that makes one.**
//!
//! Everything here is a pure function of a slice of samples: no decoder, no device, no `App`. That is deliberate and
//! it is the same rule [`crate::model`] follows — the half of the feature that can be tested with no hardware is the
//! half that decides the numbers, and it is where a defect would otherwise hide behind a plausible vector.
//!
//! # What the model wants, measured
//!
//! [#2's spike](../../docs/loop/runs/2026-11-03-62/research.md) fed CLAP's feature extractor 1 s, 4 s and 10 s clips at
//! 48 kHz. Every one produced **1001 mel frames** from a `[batch, 4, 1001, 64]` tensor: the processor resamples,
//! pads and windows to a fixed shape. So this module's job is not to shape a tensor — it is to hand the extractor a
//! **48 kHz mono buffer**, and to say plainly what it did with everything that is not one.

use super::MODEL_SAMPLE_RATE;

/// Why a buffer could not be prepared for the model.
///
/// A failure here must be one of these, never a silently padded or truncated buffer: a vector built from the wrong
/// samples is indistinguishable from a good one downstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferProblem {
    /// The input's sample rate is zero, so any conversion would divide by it.
    ZeroSampleRate,
    /// The input has no channels, so a downmix would divide by zero.
    NoChannels,
    /// The input produced no samples at all after downmixing.
    Empty,
    /// The input is at a rate this crate cannot convert to the model's rate **without aliasing**.
    ///
    /// This is a refusal rather than a conversion because the alternative is worse: a linearly interpolated or
    /// sample-dropping conversion produces a plausible embedding from a corrupted signal, and nothing downstream can
    /// tell the difference. `docs/loop/needs-decision/semantic-dependencies.md` records the measurement that rules out
    /// `rodio`'s converter and the two remaining options.
    ResamplingRequired {
        /// The rate the file actually has.
        from: u32,
        /// The rate the model needs: [`super::MODEL_SAMPLE_RATE`].
        to: u32,
    },
}

impl core::fmt::Display for BufferProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            BufferProblem::ZeroSampleRate => "the sample rate is 0 Hz",
            BufferProblem::NoChannels => "the file declares 0 channels",
            BufferProblem::Empty => "the file decoded to no samples",
            BufferProblem::ResamplingRequired { from, to } => {
                return write!(
                    f,
                    "the file is {from} Hz and the model needs {to} Hz; no resampler is approved yet \
                 (docs/loop/needs-decision/semantic-dependencies.md)"
                );
            }
        };
        f.write_str(s)
    }
}

/// **Downmix interleaved samples to mono** by averaging each frame's channels.
///
/// Averaging, not summing, so a stereo file and its mono fold-down have the same amplitude — a sum would put every
/// stereo asset 6 dB above every mono one and the loudness filter in `R-03` would then be comparing two scales.
///
/// A non-finite sample becomes `0.0`: a decoder that produced NaN has already failed, and propagating it would make
/// every subsequent comparison false rather than false-looking.
pub fn downmix_to_mono(interleaved: &[f32], channels: u16) -> Result<Vec<f32>, BufferProblem> {
    if channels == 0 {
        return Err(BufferProblem::NoChannels);
    }
    let ch = channels as usize;
    if interleaved.len() < ch {
        return Ok(Vec::new());
    }
    let frames = interleaved.len() / ch;
    let inv = 1.0 / ch as f32;
    let mut out = Vec::with_capacity(frames);
    for frame in interleaved.chunks_exact(ch).take(frames) {
        let mut sum = 0.0f32;
        for &s in frame {
            sum += if s.is_finite() { s } else { 0.0 };
        }
        out.push(sum * inv);
    }
    Ok(out)
}

/// **Prepare decoded audio for the model**: downmix to mono and hand back a buffer at
/// [`super::MODEL_SAMPLE_RATE`].
///
/// One function, one path from a decoded file to a model input. A file already at the model's rate is converted to
/// mono and returned; **a file at any other rate is refused** with [`BufferProblem::ResamplingRequired`], carrying both
/// rates.
///
/// That refusal is the crate's fail-loud rule at the exact place it matters most. A 44.1 kHz asset is not an error in
/// itself — it is the commonest rate a game ships — so the message names it and points at the decision file rather than
/// blaming the file. When an approved resampler lands, this function gains the conversion and the variant stops being
/// returned; nothing else changes.
pub fn model_buffer(interleaved: &[f32], channels: u16, sample_rate: u32) -> Result<Vec<f32>, BufferProblem> {
    if sample_rate == 0 {
        return Err(BufferProblem::ZeroSampleRate);
    }
    if sample_rate != MODEL_SAMPLE_RATE {
        return Err(BufferProblem::ResamplingRequired { from: sample_rate, to: MODEL_SAMPLE_RATE });
    }
    let mono = downmix_to_mono(interleaved, channels)?;
    if mono.is_empty() {
        return Err(BufferProblem::Empty);
    }
    Ok(mono)
}

/// The peak absolute sample of a buffer, for a caller that wants to know whether a file was silent before spending a
/// GPU pass on it.
pub fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().fold(0.0f32, |m, s| if s.is_finite() { m.max(s.abs()) } else { m })
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`BufferProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for BufferProblem {}
