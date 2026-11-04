//! **A file, decoded to the buffer a model can consume.**
//!
//! `buffer.rs` owns the arithmetic — downmix, rate agreement — and this module owns the one thing that needs a
//! decoder: turning a path into interleaved samples plus the rate and channel count the header promised.
//!
//! # One decoder, and it is one the crate already has
//!
//! `rodio` is already a dependency behind `mixer`, and it decodes **wav, vorbis, flac and mp3** through the features this crate
//! enables — exactly the formats `library::AUDIO_EXTENSIONS` calls audio. The semantic feature therefore **does not bring a
//! second decoder**: an asset indexed by `semantic` and an asset played by `mixer` are decoded by the same code, so a file that
//! plays is a file that indexes. (That list and these features were once two unrelated statements, and `.mp3` and `.flac` were
//! scanned as audio and then refused; `tests/semantic_decode.rs` now decodes a real file of every extension on the list.)
//!
//! **Vorbis is `lewton`, the decoder Bevy's own `vorbis` feature selects** — and until iteration 115 it was `symphonia`'s, which
//! made a loop of any Ogg silent (`Cargo.toml` has the measurement). Against `libvorbis` on 254 real Ogg effects `lewton` is within
//! 7 × 10⁻⁷ of it on every file, **including the 25 hot masters that decode past ±1**; `symphonia` clamped those at ±1 and was off by
//! up to 0.30. The Kenney golden benchmark with the real CLAP model is unchanged to four decimals on every metric, so the swap
//! moved no ranking (`docs/loop/runs/2026-11-03-115/evidence/after/goldenbench-kenney-lewton.txt`).
//!
//! **One correction is made on the way in**: an Ogg stream is cut to the length its last page declares (`decode`'s doc), because
//! the decoders keep the padding of the last Vorbis block — `lewton` for 51 of those 254 files (up to 936 frames, 21 ms at
//! 44.1 kHz), `symphonia` built with `Decoder::new` for 253 (up to 1 019) — and the index would have *ranked* it. The playback paths cut it too, **for a loop
//! only** — the padding is what sits between two passes, where a one-shot just ends in it — and both read the length through one
//! walk of the pages, `crate::ogg`, so what is indexed and what loops cannot disagree about where a stream ends.
//!
//! **What it costs — and an earlier version of this paragraph was wrong.** It said the decoder *"costs no new crate"* because
//! `bevy_audio` already resolves `rodio 0.22.2` (`cargo tree -i rodio`: `bevy_audio → bevy_internal → bevy → bevy_siren`). `rodio`
//! is in the default graph; **its decoders are not** — no `symphonia` or `lewton` crate is — so enabling the feature adds them: **18 of
//! the 36 crates `semantic` adds** (eight of them `symphonia`'s, for `wav`, `flac` and `mp3`; `lewton`, `ogg`, `byteorder` and `tinyvec`
//! for Vorbis), against 6 for `ort`, 7 for `rubato`, 3 for `ebur128` and 4 for `serde_json` (two crates are shared). Measured with
//! `cargo tree --edges normal`: the default build is **413 lines, 149 crates, unchanged**; `semantic` is **493 lines, 185 crates**
//! (497 and 183 with `symphonia`'s Vorbis decoder, which is two crates, `-codec-vorbis` and `-format-ogg`, against `lewton`'s four).
//! That is **this crate's own graph**, which builds `bevy` with its defaults off; a game's graph already holds whatever its `bevy`
//! enabled — `lewton` and its three, by default — so the decoder crates `semantic` adds *there* are the formats the game does not
//! already read, and **none for Vorbis**.
//!
//! # The rate
//!
//! A file at the model's rate is downmixed and returned; a file at any other rate is downmixed and **resampled** by
//! [`super::resample`], which is the commonest case in a game's library. `buffer::model_buffer` still refuses a rate it
//! cannot convert, and that remains the low-level contract — but nothing on this path reaches it with a rate other than
//! the model's, because the conversion happens first.

use std::fs::File;
use std::io::Read;
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
///
/// **An Ogg stream is cut to the length its last page declares.** The decoder keeps the padding of the stream's last block, which the
/// Vorbis spec says to discard, so an Ogg effect came back up to 20 ms long — and CLAP, which repeats a short clip to fill its input,
/// ranked those effects differently from the same sound decoded exactly (`ogg_declared_frames` has the measurement).
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
    let mut samples: Vec<f32> = decoder.collect();
    if let Some(declared) = ogg_declared_frames(path) {
        let frames = samples.len() / usize::from(channels);
        if declared < frames && frames - declared <= MAX_END_TRIM_FRAMES {
            samples.truncate(declared * usize::from(channels));
        }
    }
    Ok(Decoded { samples, sample_rate, channels })
}

/// The most frames a decoder can return past the end of an Ogg stream and still be that stream's unfinished last block. **Design
/// default**: the longest block Vorbis allows is 2^13 = 8192 frames (Vorbis I, §4.2.2) and the excess is at most half of one, so a
/// surplus larger than this is not a block's padding but something else, and is left alone.
const MAX_END_TRIM_FRAMES: usize = 8192;

/// **The number of frames an Ogg stream says it has**: the granule position of its last page — when the file is **one** logical stream
/// whose pages run unbroken to the end of the file, the last is marked end-of-stream (RFC 3533, §6), and the packet count can account for
/// the position (`crate::ogg::inspect` has the window). `None` for anything else: not Ogg, a chained file (two streams back to back have
/// no single length), a stream cut off before its end, a position of −1 (*no packet finished on this page*). The caller cuts only on a
/// length the file actually stated.
///
/// **Why it is needed.** A Vorbis stream ends on a page whose granule position is the number of frames the stream has, and the final
/// packet decodes to a whole block, which is more; the spec says to discard the excess (Vorbis I, A.2). Neither `rodio` Vorbis decoder
/// does it for every file: against `libvorbis` on Kenney's 254 CC0 Ogg effects `lewton` returned more for **51** (up to 936 frames, a padding peak as
/// high as −6 dBFS) and `symphonia`, built with `Decoder::new`, for **253** (up to 1 019) — `docs/loop/runs/2026-11-03-115/evidence/after/vorbis-backend-parity.txt`. On
/// iteration 112's 209 files, built from a `File` as this function builds it, `symphonia` was **59 up to 975 frames long** (20 ms at 48 kHz), none short. On the RPG pack's effects (0.27 – 1.5 s)
/// that tail alone moved a similarity by up to 0.085 and changed the top three of **all 12** descriptions tried; cut to the declared length
/// the worst difference was 0.004, the top three differed for 3 of 12 and the rank correlation was never below 0.998
/// (`docs/loop/runs/2026-11-03-112/evidence/after/formats-parity-ogg-vs-libvorbis.txt`, `ogg-tail-vs-clamp.txt`). MP3 and FLAC arrive
/// at their exact length and are not
/// touched — the same measurement found MP3 identical to `ffmpeg`'s decode on all 259 pairs.
///
/// The file is read a second time only when it starts with `OggS`: a WAV, FLAC or MP3 costs four bytes.
fn ogg_declared_frames(path: &Path) -> Option<usize> {
    let mut magic = [0u8; 4];
    File::open(path).ok()?.read_exact(&mut magic).ok()?;
    if &magic != b"OggS" {
        return None;
    }
    usize::try_from(crate::ogg::inspect(&std::fs::read(path).ok()?)?.frames).ok()
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
