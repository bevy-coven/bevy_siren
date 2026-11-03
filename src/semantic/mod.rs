//! **Semantic selection**: describe a sound in words, and find the sounds that fit it.
//!
//! Behind the `semantic` feature and off by default. The feature turns a library of audio into an index that can be
//! queried by description, by another sound, or by both — and it does so without changing anything a user who only
//! wants playback builds.
//!
//! # What is here now, and what is not
//!
//! The pipeline is built in dependency order, so this module is deliberately thin: the pieces that need no model
//! exist and are tested, and the pieces that need one are added behind it. Nothing here is a stub — a function either
//! does its job or is not exported yet.
//!
//! | Stage | State |
//! |---|---|
//! | scan a directory into a manifest | [`crate::library`] — always available, no feature needed |
//! | decode and resample to the model's rate | in progress |
//! | embed with CLAP's audio tower | measured (`docs/adr/0002-inference-runtime.md`) |
//! | index and query | not yet |
//!
//! # The one rule this module inherits
//!
//! **A number without its source, or the words *design default*, is a magic constant.** Every rate, window and
//! threshold this feature adds is a named field on a serde type, with its default either citing the measurement or the
//! standard it comes from, or saying plainly that it is this crate's own choice.

pub mod buffer;

/// The sample rate CLAP's feature extractor resamples to before it builds a mel spectrogram.
///
/// 48 kHz is the model's own input rate, read from its feature extractor rather than assumed: the processor produced
/// `[batch, 4, 1001, 64]` from 48 kHz audio and padded every duration to the same 1001 frames, measured in
/// `runs/2026-11-03-62/research.md`.
pub const MODEL_SAMPLE_RATE: u32 = 48_000;

/// The number of mel frames CLAP's processor emits for any clip, whatever its duration.
///
/// **Measured**: 1 s, 4 s and 10 s clips all produced 1001 frames, because the processor resamples and pads to the
/// model's fixed window. That is why chunking a long track is a counting problem rather than a shaping problem.
pub const MODEL_MEL_FRAMES: usize = 1001;

/// The dimension of a CLAP audio embedding.
///
/// **Measured**: `get_audio_features` returns `[batch, 512]`, and it **L2-normalises** it before returning, so a
/// cosine between two of them is a dot product of unit vectors.
pub const EMBEDDING_DIM: usize = 512;
