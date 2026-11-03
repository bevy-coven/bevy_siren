# Iteration 93 — perf

## Indexing: a real regression, measured twice

`cargo run --release --features semantic --example indexbench -- <dir> 200`, Apple M4, `--release`.

| | iteration 89–92 | **iteration 93** |
|---|---|---|
| decode | **~1 150 clips/s** | **419.8 / 568.8 clips/s** (two runs) |
| 5 000 clips | 4 s | **13 s** |

**Decode fell 2.0–2.7×**, and the cause is exactly the two passes this iteration wired in: tempo's autocorrelation over
~400 onset frames per track, and key's **8192-point** transform per chroma frame. Neither shares arithmetic with the mel
path, so the buffer is decoded once and then walked three times.

**Reported as the headline rather than a footnote.** This is the case the iteration targeted — a build now yields two
more features per asset — so it is a trade, not a leak; and 13 s for 5 000 clips is still far below the ~3 minutes the
audio tower itself would take.

**But it is the third iteration in a row where a feature added a pass**, and the fix is now obvious: one shared spectral
pass producing mel, centroid, chroma and onset from the same FFT. That is the next iteration's job or the one after.

## Per-frame: unchanged

| voices | µs/frame | iteration 92 |
|---|---|---|
| 64 | **0.323** | 0.107 |
| 256 | **1.119** | 0.833 |

No audio-path change; inside the ranges this code has shown across iterations 80–93.
