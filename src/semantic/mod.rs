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

pub mod ask;
pub mod budget;
pub mod buffer;
pub mod decode;
pub mod eval;
pub mod index;
pub mod indexer;
pub mod model;
pub mod query;
pub mod rank;
pub mod resample;
pub mod search;
pub mod session;
pub mod tokenizer;
pub mod weights;

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

/// The version this crate's measurements were taken with, and the version an index is stamped with.
///
/// It names the measurement rather than a model tag because the spike did not pin a revision: an index carrying this
/// stamp is one this crate has evidence for, and a later pinned revision gets its own string.
pub const MEASURED_CLAP_VERSION: &str = "transformers-5.22.1/torch-2.15.0/opset-17";

/// The dimension of a CLAP **text** embedding, which is the same space as the audio one.
///
/// **Measured**: `get_text_features` returns `[batch, 512]`, every row L2-normalised — so a text query and an audio
/// clip can be compared by a cosine, which is the whole premise of text-to-audio retrieval. Verified against six
/// clips and five descriptions in `runs/2026-11-03-63/research.md` S3.
pub const TEXT_EMBEDDING_DIM: usize = 512;

/// The maximum number of tokens CLAP's text tower accepts.
///
/// The tokeniser produced **8 tokens for 4 short phrases** and **15 for a 12-word sentence** in the measurement, and
/// the exported graph ran both without re-export, so the token axis is dynamic. 77 is the model card's limit for
/// `htsat-fused`; a long description must be truncated deliberately rather than silently, which is why the number is
/// here rather than inline at a call site.
pub const MAX_TEXT_TOKENS: usize = 77;

/// The dimension of a CLAP audio embedding.
///
/// **Measured**: `get_audio_features` returns `[batch, 512]`, and it **L2-normalises** it before returning, so a
/// cosine between two of them is a dot product of unit vectors.
pub const EMBEDDING_DIM: usize = 512;
