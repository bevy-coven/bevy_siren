# Iteration 92 — perf

## Per-frame: unchanged

| voices | µs/frame | iteration 91 |
|---|---|---|
| 64 | **0.107** | 0.282 |
| 256 | **0.833** | 1.055 |

No audio-path change; inside the ranges this code has shown across iterations 80–92.

## Indexing: unchanged this iteration

`semantic::key` is not called by `build` — the wiring waits for one schema change that carries tempo and key together.

## The cost key will add, and it is the largest of these features

The chroma uses an **8192-point** transform, four times the mel window and twice tempo's. Per frame that is `n log n`
over 8192 rather than 1024 — roughly **10× the arithmetic of a mel frame** — and the corpus test shows it: 12 key tests
now take **1.1 s** against tempo's 11 s for 80 tracks, so per track the two are comparable.

That is not measured as clips/s because it is not in the build path. When tempo and key land there, `indexbench` will
show it — and iteration 89's precedent (a 55 % decode regression from one extra pass) says it will be the headline, which
is the argument for **one shared chroma/mel pass** rather than three.

**The honest limit**: the per-track cost of key is *not* measured here. The corpus test's 1.1 s covers 24 tracks plus
the test harness, which is a harness number rather than a per-clip figure; `indexbench` is where it belongs and it is not
in the build path yet. Writing a derived number here would be a guess dressed as a measurement.
