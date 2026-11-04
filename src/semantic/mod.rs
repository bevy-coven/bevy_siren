//! **Semantic selection**: describe a sound in words, and find the sounds that fit it.
//!
//! Behind the `semantic` feature and off by default. The feature turns a library of audio into an index that can be
//! queried by description, by another sound, or by both — and it does so without changing anything a user who only
//! wants playback builds.
//!
//! # What is here
//!
//! The pipeline was built in dependency order, so most of it needs no model: only `model` (this module's, not [`crate::model`])
//! touches `ort`, and everything else takes an embedding as a slice and is tested with no device, no weights and no runtime.
//! Nothing here is a stub — a function either does its job or is not exported.
//!
//! | Stage | Modules |
//! |---|---|
//! | scan a directory into a manifest | [`crate::library`] — always available, no feature needed |
//! | decode, downmix and resample to the model's rate | `decode`, `buffer`, `resample` |
//! | the features the audio tower reads | `mel` (CLAP's log-mel, to 0.0545 dB of the reference), `chunk` (long tracks) |
//! | embed a sound or a description | `model`, `tokenizer`, `query`, `ask` |
//! | store, update, say where it came from | `index`, `indexer`, `incremental`, `provenance` |
//! | rank, filter and weigh | `rank`, `search`, `filter`, `ensemble`, `weights` |
//! | what the sound itself says | `loudness`, `spectral`, `tempo`, `key`, `reverb`, `analysis` |
//! | the workflow as calls, and the MCP host that serves them | `tools`, `session`, `server` |
//! | measure it | `eval`, `golden` (labelled sets), `budget` (the GPU budget) |
//!
//! # The one rule this module inherits
//!
//! **A number without its source, or the words *design default*, is a magic constant.** Every rate, window and
//! threshold this feature adds is a named field on a serde type, with its default either citing the measurement or the
//! standard it comes from, or saying plainly that it is this crate's own choice.

pub mod analysis;
pub mod ask;
pub mod budget;
pub mod buffer;
pub mod chunk;
pub mod decode;
pub mod ensemble;
pub mod eval;
pub mod filter;
pub mod golden;
pub mod incremental;
pub mod index;
pub mod indexer;
pub mod key;
pub mod loudness;
pub mod mel;
pub mod model;
pub mod provenance;
pub mod query;
pub mod rank;
pub mod resample;
pub mod reverb;
pub mod search;
pub mod server;
pub mod session;
pub mod spectral;
pub mod tempo;
pub mod tokenizer;
pub mod tools;
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
