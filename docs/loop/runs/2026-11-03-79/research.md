# Iteration 79 — research: the vector store, sized and timed rather than guessed

The item is [#5](https://github.com/bevy-coven/bevy_siren/issues/5): *"ADR: vector index and metadata store."* p0,
nothing blocks it, blocks #10 and #15.

## Sources

| Source | The number it reports |
|---|---|
| this crate, `src/semantic/index.rs::to_text` | the record line is `{hash:016x}` then ` {v:?}` per float — `{:?}` on `f32` is Rust's **shortest round-trip** form |
| this crate, `src/semantic/mod.rs` | `EMBEDDING_DIM = 512`, `MODEL_SAMPLE_RATE = 48_000` |
| this crate, `src/library.rs` | the content hash is **FNV-1a, 64-bit**, over the file bytes; `Asset` also carries `bytes` and `paths` |
| this crate, `src/semantic/index.rs` | `MAGIC = "bevy_siren index 1"`; header carries count and a **checksum over the body** (`fnv1a`) |
| measured this iteration, 2 000 synthetic 512-d unit vectors through the same formatting | **6 376.8 mean bytes per text record** |
| measured this iteration, BLAS `f32` dot over 5 000 × 512 | **0.05–0.16 ms** steady state, 1.87 ms first call |
| measured this iteration, `f16` dot, same shape | 4.90 ms; top-10 identical to `f32` on that query |

## The technique, in the sources' terms

An embedding index is keyed by something stable and scanned by a dot product. Two decisions dominate everything else:

1. **What the key is.** A path is unstable under rename and re-encode; a content hash is stable under rename and
   deliberately *not* under re-encode. The crate already made that choice in `library.rs` — `paths` is a `Vec` because
   one content can be at several paths, and `hash` is the identity — so the store inherits the decision rather than
   making a second one.
2. **How the scan is paid for.** Brute force is `O(n·d)` per query; an approximate structure trades exactness for
   sublinearity. The trade only pays when the linear cost exceeds a budget, and the budget here is one game frame.

## Parameters these measurements report

| Parameter | Value | Where |
|---|---|---|
| dimension | **512** | `EMBEDDING_DIM`, measured from `get_audio_features` |
| text bytes per record | **6 376.8** mean | measured over 2 000 records |
| binary bytes per record | 16 hash + 512 × 4 = **2 064** (`f32`); **1 040** (`f16`) | arithmetic on the measured dimension |
| 5 000-clip file | **31.88 MB** text / **10.30 MB** `f32` / **5.20 MB** `f16` | computed |
| query, 5 000 × 512, `f32` | **0.05–1.87 ms** | measured |
| query, 50 000 (extrapolated) | **0.5–19 ms** | linear scaling, labelled as extrapolation |

## What good sounds like

A store whose numbers are **computed at the size the answer depends on**, not at a plausible one. The issue's own text
proposes "a 5 MB index … or a 40 MB SQLite file"; the measurement says the crate's *existing* text format is **31.9 MB**
at 5 000 clips, so the 40 MB figure is the format it already has, and 5 MB requires `f16`. An ADR that repeated the
issue's numbers without measuring would have chosen a store to avoid a size the crate is already at.

The read path should need nothing but `open` and `read`: a game ships an index as data and ranks from a read-only
filesystem, and the only writable operation is the one the editor performs.
