# eval.md — iteration 75

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 9.8 s (warm) |
| `cargo test` | green — **198** + 6 |
| `cargo test --all-features` | green — **367** |

**Nothing red.** No case in `cues.yaml` red. Working tree clean at `bf52554`.

## Machine state

| | |
|---|---|
| start | **1.0 GB free**, `target` ~14 GB |
| after `cargo clean` | **14 GB free** (13.8 GiB removed) |
| when the render ran | **2.5 GB** — the release binary was already built, so it took **31.7 s** rather than nine minutes |

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged; case 31 green since iteration 72 |

## After

`bevy_siren`: **198 default / 368 all-features**, clippy 0, fmt, rustdoc — green.

**The visual gate holds**: 32 render files, **14 distinct** cue hashes, **−21.5 LUFS** integrated, **−7.5 dBFS** true
peak — identical to iterations 67, 71 and 74, so neither the mel path nor the documentation touched the audio.

## What this iteration did that the gate cannot show

Two items, one code and one writing:

1. **`indexer::build_from_model`** — the closure that joins the mel path to the audio tower, so a folder becomes a
   stamped index with no caller-supplied embedder.
2. **`docs/guide/semantic.md`** and a README line — the semantic layer had **zero** mentions before this iteration.

The second is why `eval.md` records it: a feature nobody can find does not ship, and the loop's own rule is that a
feature is not done until its README line exists.
