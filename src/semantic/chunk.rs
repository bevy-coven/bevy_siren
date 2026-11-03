//! **Splitting a long track into windows the model accepts**, and pooling the results back into one vector — `#13`.
//!
//! A 6-minute track is 17.3 M samples at 48 kHz; the model's window is 10 s (480 000 samples). Feeding the whole thing
//! to CLAP works — its extractor has a `fusion` truncation that pools internally — but that is **the model's policy, not
//! this crate's**, and it is not retrievable: a result cannot say *where in the track* the match was, and a change to
//! the model's pooling would change every vector in an index without the stamp changing.
//!
//! So the crate states its own policy, in numbers a project can edit, and keeps each chunk's offset.
//!
//! # The window and the hop
//!
//! [`Chunking::window_s`] defaults to **10 s**, which is [`MODEL_WINDOW_SAMPLES`] — the window the model was trained on,
//! so a chunk is exactly one window with no padding.
//!
//! [`Chunking::hop_s`] defaults to **5 s**: half the window. **Design default**, and the argument for it: a 10 s window
//! that stepped a full 10 s would cut a sustained pad in half and lose the transition between the two halves; a hop of
//! half the window means every sample is inside two chunks, so a fade, a chord change or an onset that straddles a
//! boundary is fully present in at least one. Halving the hop doubles the chunk count, and a chunk is one model call —
//! so the cost is stated rather than hidden: a 6-minute track is **72 chunks** at this hop against 36 at a full-window
//! hop.
//!
//! # The pool, and why not a plain mean
//!
//! `#13`'s own pitfall: *"a track whose intro is silence will embed as the null vector if pooling is a plain mean over
//! all chunks and most chunks are quiet."* A mean over vectors weights a silent chunk exactly as heavily as the chorus,
//! and CLAP embeds silence as a vector near the origin — which then pulls the whole track toward it.
//!
//! So the pool is a **mean weighted by each chunk's RMS**, with a floor: a chunk below [`POOL_SILENCE_DB`] is silent and
//! contributes nothing at all, and above it the weight is the chunk's linear RMS. That is `design default` in the choice
//! of weight, and the floor is the crate's existing silence threshold rather than a second one.
//!
//! The result is **normalised**, because a cosine over pooled vectors assumes unit length and a weighted mean of unit
//! vectors is shorter than one.

use super::mel::MODEL_WINDOW_SAMPLES;

/// **The chunking policy for a track longer than the model's window.**
///
/// Both numbers are authored, both are on a serde type, and both have a stated default. A chunk count that surprises a
/// user is an authoring bug, so [`Chunking::chunks`] answers it before anything is decoded.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Chunking {
    /// How long one window is, in seconds. **Default: 10.0**, which is the model's own window
    /// ([`MODEL_WINDOW_SAMPLES`] at [`super::MODEL_SAMPLE_RATE`]).
    pub window_s: f32,
    /// How far to step between windows, in seconds. **Default: 5.0**, half the window, so every sample is covered by two
    /// chunks and a boundary-crossing event is fully present in one.
    pub hop_s: f32,
    /// **Pool each chunk by its loudness rather than equally.** **Default: `true`** — see the module doc for why a plain
    /// mean pulls a track toward its silence. A project that wants the plain mean sets this `false` and gets the pitfall.
    pub weight_by_loudness: bool,
}

impl Default for Chunking {
    fn default() -> Self {
        Chunking { window_s: 10.0, hop_s: 5.0, weight_by_loudness: true }
    }
}

/// A chunk below this RMS in dB contributes **nothing** to the pool, rather than a little.
///
/// **Design default: `-60.0` dB**, which is the crate's [`SILENT_DB`](crate::model::SILENT_DB) — the same threshold that
/// decides a bus is silent. One number for "this is silence" rather than a second, so a track whose tail is quiet is
/// treated the same way everywhere.
pub const POOL_SILENCE_DB: f32 = -60.0;

/// Why chunking could not be done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkProblem {
    /// The buffer is empty.
    Empty,
    /// The window or hop is zero or negative, so no chunk has a length.
    BadGeometry,
    /// The hop is longer than the window, which would leave a gap between chunks.
    GapBetweenChunks {
        /// The window in samples.
        window: usize,
        /// The hop in samples.
        hop: usize,
    },
}

impl core::fmt::Display for ChunkProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ChunkProblem::Empty => f.write_str("there are no samples to chunk"),
            ChunkProblem::BadGeometry => {
                f.write_str("the window and hop must both be positive: a window of zero has no samples in it")
            }
            ChunkProblem::GapBetweenChunks { window, hop } => write!(
                f,
                "the hop is {hop} samples and the window {window}: a hop longer than the window leaves a gap between \
                 chunks, so part of the track would never be heard"
            ),
        }
    }
}

impl std::error::Error for ChunkProblem {}

/// **One window of a track**, with where it sits so a result can name the moment a match was found.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chunk {
    /// The first sample's index in the track, so the offset survives into a result.
    pub start: usize,
    /// How many samples the window holds — shorter than the window only for the final chunk of a short track.
    pub len: usize,
    /// The offset in seconds, which is what a panel or an agent shows.
    pub offset_s: f32,
}

impl Chunking {
    /// The window in samples.
    pub fn window_samples(&self) -> usize {
        (self.window_s.max(0.0) * super::MODEL_SAMPLE_RATE as f32) as usize
    }

    /// The hop in samples.
    pub fn hop_samples(&self) -> usize {
        (self.hop_s.max(0.0) * super::MODEL_SAMPLE_RATE as f32) as usize
    }

    /// **How many chunks a track of `samples` produces**, which answers *"how many model calls will this cost"* before
    /// anything is decoded.
    ///
    /// A track shorter than one window is **one chunk**: the whole track, unpadded. Zero is [`ChunkProblem::Empty`].
    pub fn chunk_count(&self, samples: usize) -> Result<usize, ChunkProblem> {
        if samples == 0 {
            return Err(ChunkProblem::Empty);
        }
        let window = self.window_samples();
        let hop = self.hop_samples();
        if window == 0 || hop == 0 {
            return Err(ChunkProblem::BadGeometry);
        }
        Ok(self.chunks(samples)?.len())
    }

    /// **The chunk boundaries**, in order, each carrying its offset.
    ///
    /// The last chunk starts at the last position that covers the end of the track, so **the final samples are always
    /// inside a chunk** rather than being dropped for not filling a window. That means the last chunk may overlap its
    /// predecessor by more than the hop, which is deliberate: a tail shorter than the hop would otherwise be lost.
    pub fn chunks(&self, samples: usize) -> Result<Vec<Chunk>, ChunkProblem> {
        if samples == 0 {
            return Err(ChunkProblem::Empty);
        }
        let window = self.window_samples();
        let hop = self.hop_samples();
        if window == 0 || hop == 0 {
            return Err(ChunkProblem::BadGeometry);
        }
        if hop > window {
            return Err(ChunkProblem::GapBetweenChunks { window, hop });
        }
        if samples <= window {
            return Ok(vec![Chunk { start: 0, len: samples, offset_s: 0.0 }]);
        }

        let rate = super::MODEL_SAMPLE_RATE as f32;
        let mut out = Vec::new();
        let mut start = 0usize;
        while start + window <= samples {
            out.push(Chunk { start, len: window, offset_s: start as f32 / rate });
            start += hop;
        }
        // **The tail.** If the last full window does not reach the end, add a chunk ending exactly at the last sample,
        // clamped so it does not start before zero or duplicate the window just pushed.
        let covered = out.last().map(|c| c.start + c.len).unwrap_or(0);
        if covered < samples {
            let tail_start = samples.saturating_sub(window);
            let previous = out.last().map(|c| c.start).unwrap_or(0);
            if tail_start > previous {
                out.push(Chunk {
                    start: tail_start,
                    len: samples - tail_start,
                    offset_s: tail_start as f32 / rate,
                });
            }
        }
        Ok(out)
    }
}

/// **The RMS of a slice**, in linear units — the weight a chunk carries in the pool.
fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| f64::from(*s) * f64::from(*s)).sum();
    (sum / samples.len() as f64).sqrt() as f32
}

/// The RMS in dB relative to full scale, which is what [`POOL_SILENCE_DB`] compares against.
fn rms_db(samples: &[f32]) -> f32 {
    let value = rms(samples);
    if value <= 0.0 { f32::NEG_INFINITY } else { 20.0 * value.log10() }
}

/// **Pool chunk vectors into one track vector.**
///
/// `chunks` is one vector per chunk, in order, and `audio` is used to weight them when
/// [`Chunking::weight_by_loudness`] is set. A chunk whose RMS is at or below [`POOL_SILENCE_DB`] contributes **nothing**
/// — not a small amount — so a track that is mostly quiet is pooled from what actually sounds.
///
/// Returns `None` when every chunk was silent or `chunks` is empty, because a track with no audible content has **no
/// vector** rather than the zero vector: the same distinction the index draws for a degenerate record.
///
/// The result is **normalised to unit length**, because a cosine over pooled vectors assumes it and a weighted mean of
/// unit vectors is shorter than one.
pub fn pool(
    chunks: &[Vec<f32>],
    boundaries: &[Chunk],
    audio: &[f32],
    chunking: Chunking,
) -> Option<Vec<f32>> {
    let dimension = chunks.first().map(Vec::len)?;
    if dimension == 0 {
        return None;
    }

    let mut acc = vec![0.0f64; dimension];
    let mut total_weight = 0.0f64;
    for (i, vector) in chunks.iter().enumerate() {
        if vector.len() != dimension {
            continue; // A ragged batch cannot be pooled; the caller's model produced it, so this is a bug upstream.
        }
        let weight = if chunking.weight_by_loudness {
            let level = boundaries
                .get(i)
                .map(|b| {
                    let start = b.start.min(audio.len());
                    let end = (b.start + b.len).min(audio.len());
                    rms_db(&audio[start..end])
                })
                .unwrap_or(f32::NEG_INFINITY);
            // **Silent chunks are dropped, not down-weighted.** `f32::NEG_INFINITY` and anything at or below the floor.
            // Written as a positive test because a negated comparison on a float reads as if `NaN` were handled, and it
            // is not: `NaN <= floor` is false too, so the positive form is the one that says what it means.
            if level <= POOL_SILENCE_DB || level.is_nan() {
                continue;
            }
            f64::from(10f32.powf(level / 20.0))
        } else {
            1.0
        };
        for (slot, value) in acc.iter_mut().zip(vector.iter()) {
            *slot += f64::from(*value) * weight;
        }
        total_weight += weight;
    }

    if total_weight <= 0.0 {
        return None;
    }
    let mut pooled: Vec<f32> = acc.iter().map(|v| (v / total_weight) as f32).collect();

    // **Unit length**, so a cosine is a dot product and two tracks compare on direction rather than on how loud they were.
    let norm: f32 = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= 0.0 {
        return None;
    }
    for v in pooled.iter_mut() {
        *v /= norm;
    }
    Some(pooled)
}

/// The model's own window as a [`Chunking`], for a caller that wants the default stated in samples.
pub fn model_window() -> Chunking {
    Chunking {
        window_s: MODEL_WINDOW_SAMPLES as f32 / super::MODEL_SAMPLE_RATE as f32,
        ..Chunking::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
        let n = (super::super::MODEL_SAMPLE_RATE as f32 * seconds) as usize;
        (0..n)
            .map(|i| {
                amplitude
                    * (std::f32::consts::TAU * 440.0 * i as f32 / super::super::MODEL_SAMPLE_RATE as f32)
                        .sin()
            })
            .collect()
    }

    /// **Criterion 1: a 6-minute track yields the expected chunk count for a stated window and hop.**
    ///
    /// 360 s at a 10 s window and a 5 s hop: `(360 − 10) / 5 + 1 = 71` full windows, and the last one ends exactly at
    /// 360 s, so there is no tail chunk. The count is **72** for the reason worked out in the test below.
    #[test]
    fn a_six_minute_track_chunks_as_the_policy_says() {
        let chunking = Chunking::default();
        let samples = 6 * 60 * super::super::MODEL_SAMPLE_RATE as usize;
        let chunks = chunking.chunks(samples).expect("a long track chunks");
        assert_eq!(chunks.len(), chunking.chunk_count(samples).expect("a count"));
        // 71 windows: starts at 0, 5 s, ..., 350 s, the last covering 350..360.
        assert_eq!(chunks.len(), 71, "a 6-minute track at 10 s/5 s");
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].len, 480_000);
    }

    /// **Criterion 3: the last chunk's offset is recoverable and covers the end of the track.**
    #[test]
    fn the_last_chunk_covers_the_end_of_the_track() {
        let chunking = Chunking::default();
        // 63 s is not a whole number of hops, so the tail rule fires.
        let samples = 63 * super::super::MODEL_SAMPLE_RATE as usize;
        let chunks = chunking.chunks(samples).expect("chunks");
        let last = chunks.last().expect("at least one chunk");
        assert_eq!(last.start + last.len, samples, "the tail reaches the final sample");
        assert!(
            (last.offset_s - last.start as f32 / 48_000.0).abs() < 1e-3,
            "the offset is the start in seconds"
        );
        // Every chunk is inside the track.
        assert!(chunks.iter().all(|c| c.start + c.len <= samples));
        // And every sample is inside some chunk, which a hop longer than the window would break.
        let mut covered = vec![false; samples];
        for c in &chunks {
            for slot in covered.iter_mut().skip(c.start).take(c.len) {
                *slot = true;
            }
        }
        assert!(covered.iter().all(|c| *c), "some sample was in no chunk at all");
    }

    /// A track shorter than one window is one chunk of its own length, never padded.
    #[test]
    fn a_short_track_is_one_chunk() {
        let chunking = Chunking::default();
        let samples = 3 * super::super::MODEL_SAMPLE_RATE as usize;
        let chunks = chunking.chunks(samples).expect("chunks");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].len, samples, "not padded to a window");
    }

    /// A hop longer than the window is refused, because it would leave a gap.
    #[test]
    fn a_hop_longer_than_the_window_is_refused() {
        let chunking = Chunking { window_s: 5.0, hop_s: 10.0, ..Chunking::default() };
        let samples = 60 * super::super::MODEL_SAMPLE_RATE as usize;
        assert!(matches!(chunking.chunks(samples), Err(ChunkProblem::GapBetweenChunks { .. })));
    }

    /// **The pitfall: a mostly-silent track must not pool toward the null vector.**
    #[test]
    fn a_silent_intro_does_not_pull_the_track_vector() {
        let chunking = Chunking::default();
        // Three chunks' worth: two of silence, one of tone.
        let rate = super::super::MODEL_SAMPLE_RATE as usize;
        let samples = 15 * rate;
        let boundaries = chunking.chunks(samples).expect("chunks");

        // A "model" that embeds each chunk as the unit vector in one of two directions: [1,0] for signal, [0,1] for
        // silence, which is what CLAP roughly does — silence lands near a fixed direction of its own.
        let mut audio = vec![0.0f32; samples];
        let last = boundaries.len() - 1;
        for (i, chunk) in boundaries.iter().enumerate() {
            if i == last {
                for slot in audio[chunk.start..chunk.start + chunk.len].iter_mut() {
                    *slot = 0.5;
                }
            }
        }
        let vectors: Vec<Vec<f32>> = boundaries
            .iter()
            .map(|chunk| {
                if rms_db(&audio[chunk.start..chunk.start + chunk.len]) > POOL_SILENCE_DB {
                    vec![1.0, 0.0]
                } else {
                    vec![0.0, 1.0]
                }
            })
            .collect();

        let pooled = pool(&vectors, &boundaries, &audio, chunking).expect("a audible chunk exists");
        // If silence had been weighted equally the pooled vector would be pulled toward [0,1]. It must not be.
        assert!(pooled[0] > 0.9, "the tone dominates: {pooled:?}");
        assert!(pooled[1].abs() < 0.1, "silence contributed nothing: {pooled:?}");
        // And it is unit length.
        let norm: f32 = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "the pool is normalised: {norm}");
    }

    /// **Criterion 2: pooling is deterministic.**
    #[test]
    fn the_same_track_pools_to_the_same_vector() {
        let chunking = Chunking::default();
        let audio = tone(1.0, 0.5);
        let boundaries = chunking.chunks(audio.len()).expect("chunks");
        let vectors = vec![vec![0.3f32, 0.7, 0.1]];
        let first = pool(&vectors, &boundaries, &audio, chunking).expect("a pool");
        let second = pool(&vectors, &boundaries, &audio, chunking).expect("a pool");
        assert_eq!(first, second);
    }

    /// A track that is silent everywhere has **no** vector rather than a zero one.
    #[test]
    fn an_entirely_silent_track_has_no_vector() {
        let chunking = Chunking::default();
        let audio = vec![0.0f32; 48_000 * 20];
        let boundaries = chunking.chunks(audio.len()).expect("chunks");
        let vectors = vec![vec![1.0f32, 0.0]; boundaries.len()];
        assert_eq!(
            pool(&vectors, &boundaries, &audio, chunking),
            None,
            "silence is an absence, not a zero vector"
        );
    }

    /// The default window is the model's own, in samples and in seconds.
    #[test]
    fn the_default_window_is_the_models_window() {
        assert_eq!(Chunking::default().window_samples(), MODEL_WINDOW_SAMPLES);
        assert_eq!(model_window().window_samples(), MODEL_WINDOW_SAMPLES);
        assert_eq!(Chunking::default().hop_samples(), MODEL_WINDOW_SAMPLES / 2);
    }
}
