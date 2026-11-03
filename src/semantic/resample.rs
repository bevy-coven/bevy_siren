//! **A mono buffer at the model's rate**, from a file that is not.
//!
//! `buffer::model_buffer` refuses a rate it cannot convert. This module is the conversion it was refusing for, so the
//! refusal stops being the answer for the commonest asset rate in a game's library.
//!
//! # Why an FFT resampler and not the obvious ones
//!
//! Measured in `needs-decision/resampler-dependency.md`:
//!
//! | Converter | A 1 kHz tone at 44.1 → 48 kHz |
//! |---|---|
//! | `rodio`'s `SampleRateConverter` | **discards samples** when down-sampling, its own doc comment says so |
//! | a hand-rolled linear interpolator | keeps the tone, band-limits badly — removed in iteration 62 |
//! | **`rubato`'s `FftFixedInOut`** | **0.9763** of the tone's energy, 15 kHz intact |
//!
//! An aliased mel spectrogram still becomes a plausible embedding, so a bad converter here would corrupt every vector
//! the index holds and fail nothing. That is why this is a dependency rather than twenty lines.
//!
//! # Two things the first probe got wrong, and both are measurable
//!
//! **The tail.** `FftFixedInOut` consumes fixed-size chunks, so a clip whose length is not a multiple of the chunk
//! leaves a tail behind. The 2026-11-03 probe measured **480 samples missing** from 0.5 s of audio — 2 % of every file,
//! silently. [`convert`] pads the final chunk and runs it, so nothing is dropped.
//!
//! **The latency.** An FFT resampler cannot produce its first output until its window is full, and `rubato` reports
//! the cost: [`Resampler::output_delay`] is **560 samples** at 44.1 → 48 kHz. Measured on a 1 kHz tone, skipping it
//! moves the maximum error against a pure reference from **1.77** — off by a whole cycle, i.e. meaningless — to
//! **0.071**, which is a real match with a little band-limiting at the edges.
//!
//! So the conversion **skips `output_delay()` and then truncates or pads to the length the rate ratio promises**. Both
//! numbers are checkable by hand: a 1.000 s input at 44.1 kHz is **exactly 48000** samples at 48 kHz, and a 1 kHz tone
//! is still 1 kHz.

use rubato::{FftFixedInOut, Resampler};

use super::buffer::BufferProblem;

/// The chunk size handed to the resampler.
///
/// 1024 is `rubato`'s own example size and is a power of two, which is what an FFT of this size wants. It is not a
/// tuning knob: the resampler's output is the same for any chunk size once the tail is flushed, and a test pins that.
const CHUNK: usize = 1024;

/// **Convert a mono buffer from one rate to another**, with the tail flushed.
///
/// The output length is `round(input_len × to / from)`, which is the number a person can check with a calculator and
/// the number a caller can rely on: a 1 s clip stays 1 s.
///
/// Returns [`BufferProblem::ZeroSampleRate`] for a zero rate at either end, and an empty buffer for empty input.
pub fn convert(mono: &[f32], from: u32, to: u32) -> Result<Vec<f32>, BufferProblem> {
    if from == 0 || to == 0 {
        return Err(BufferProblem::ZeroSampleRate);
    }
    if mono.is_empty() {
        return Ok(Vec::new());
    }
    if from == to {
        return Ok(mono.to_vec());
    }

    let mut resampler = FftFixedInOut::<f32>::new(from as usize, to as usize, CHUNK, 1)
        .map_err(|_| BufferProblem::ZeroSampleRate)?;
    let expected = ((mono.len() as f64) * (to as f64) / (from as f64)).round() as usize;

    // **The resampler's own latency**, read from the library rather than guessed. An FFT resampler cannot emit its
    // first sample until its window is full; skipping this many output samples is what makes the result line up with
    // the input. Measured at 560 samples for 44.1 -> 48 kHz with a 1024-frame chunk.
    let delay = resampler.output_delay();

    let mut out: Vec<f32> = Vec::with_capacity(expected + delay);
    let mut pos = 0usize;
    // **Keep going until there is enough output**, not until the input is spent. The resampler pads the final chunk
    // with zeros, and its own delay has to come out of the same buffer, so stopping at `mono.len()` leaves the tail
    // short: measured, a 1.000 s file produced only 47600 samples of real audio and 400 of padding, which read as a
    // 991 Hz tone instead of 1000. Feeding zero chunks until `expected + delay` samples exist closes it.
    while out.len() < expected + delay {
        let need = resampler.input_frames_next();
        // The input may be spent; `min` keeps the slice in range and the resize pads the final chunk with zeros.
        let end = (pos + need).min(mono.len());
        let mut chunk = if pos < mono.len() { mono[pos..end].to_vec() } else { Vec::new() };
        chunk.resize(need, 0.0);
        match resampler.process(&[chunk], None) {
            Ok(o) => out.extend_from_slice(&o[0]),
            Err(_) => break,
        }
        pos += need;
    }

    // Drop the resampler's startup latency, then hold the length the ratio promises.
    out.drain(..delay.min(out.len()));
    out.truncate(expected);
    // A resampler with latency can return slightly short; the expected length is the contract, so pad the last value
    // rather than return a buffer of the wrong duration.
    while out.len() < expected {
        out.push(0.0);
    }
    Ok(out)
}

/// **Convert a buffer to the model's rate**, which is the one conversion the indexer ever needs.
pub fn to_model_rate(mono: &[f32], from: u32) -> Result<Vec<f32>, BufferProblem> {
    convert(mono, from, super::MODEL_SAMPLE_RATE)
}
