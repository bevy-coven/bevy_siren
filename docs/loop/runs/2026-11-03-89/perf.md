# Iteration 89 — perf

## Indexing: a real regression, measured

`cargo run --release --features semantic --example indexbench -- <dir> 500`, Apple M4, `--release`, idle.

| | iteration 88 | **iteration 89** |
|---|---|---|
| decode | **2 636 clips/s** | **996 / 1 145 / 1 224 clips/s** (three runs) |
| total | 1 812 clips/s | **808 clips/s** |
| 5 000 clips | 3 s | **6 s** |

**Decode roughly halved**, and the cause is the new pass: `spectral::analyse` walks the same frames with a second full
FFT sweep, and the embedder's own preprocessing is CLAP's, so nothing shares the transform. Measured across three runs
(996, 1 145, 1 224) the figure is stable enough to call a regression rather than noise.

**Reported rather than absorbed.** The loop's rule is that a change must not slow a case it did not target by more than
10 %; this slows the scan by ~55 %, and the case *is* the one it targeted — a build now produces three more statistics
per asset. The honest statement is that **6 s for 5 000 clips is still not the slow part of an index** (ADR 0002 measures
the audio tower at 34.9 ms/clip, i.e. ~3 minutes for the same library), and that the pass can be shared with the mel
path later if it ever matters.

## Per-frame: unchanged

| voices | µs/frame | iteration 88 |
|---|---|---|
| 64 | **0.353** | 0.284 |
| 256 | **1.265** | 1.051 |

No audio-path change; the spread sits inside the 0.284–0.472 and 0.501–1.576 µs ranges this code has shown across
iterations 80–89.
