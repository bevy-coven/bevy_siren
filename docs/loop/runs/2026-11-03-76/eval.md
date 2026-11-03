# Iteration 76 — eval

The fixed cases in `cues.yaml` judged against the crate as it stands after the change, plus the baseline.

## The change's blast radius

This iteration touches **only** `src/semantic/` and `docs/`. It adds nothing to the audio path:

| Check | Result |
|---|---|
| files changed outside `src/semantic/` and `docs/` | **0** |
| audio-path files changed (`model.rs`, `cue.rs`, `mix.rs`, `play.rs`, `playback.rs`, `output.rs`, `music.rs`, `sounds.rs`, `log.rs`, `lib.rs`) | **0** |
| `semantic` in `default = []`? | **no** — it is not in the default feature list |
| `src/semantic` compiled by `cargo build` / `cargo test` / `cargo test --features serde|mixer`? | **no** — `#[cfg(feature = "semantic")]` |

So cases 1–33 cannot change: the code they compile is byte-identical to iteration 75's.

## The cases

| Case | Green/red | Observation | Reason |
|---|---|---|---|
| 1–17, 20–33 (audio behaviours) | **green, unchanged** | same source, same tests; `cargo test` **198 passed, 0 failed** | semantic is off by default and no audio file changed |
| 18 `problems()` sentences | green, unchanged | `cargo test` 198 passed | untouched |
| 19 perf at 8/64/256 voices | **not re-measured** | see the blocker below | a release build needs ~2 GB; the disk has 235 MB |
| the mel against the extractor | **green, new** | max diff **0.0545 dB** over 64 064 values; frame 0 **2.4 × 10⁻⁵ dB** | `tests::against_the_model` |
| the tiled window, frames 0/500/1000 | **green, new** | within 0.06 dB at each | `the_tiled_model_window_matches_the_extractors_own_output` |
| no frame at the floor | **green, new** | **0** all-floor frames, the reference's own count | `no_frame_of_the_tiled_window_falls_to_the_floor` |
| the embedding against the model | **green, new** | **cosine 0.999999995**, was **0.849973469** | measured through the exported `htsat-fused` graph |
| the graph's own fidelity, isolated | **green, new** | cosine **1.000000007** on the reference's mel | same probe, reference mel as input |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 + 6 + 4 doc-adjacent binaries |
| `cargo test --lib --features semantic` | **pass** — 12 (10 before, 2 new) |
| `cargo build --features semantic` | **pass** |
| `cargo fmt --check` | **pass** |
| `cargo test --features serde` | **BLOCKED — disk**, `error writing dependencies … error 28`, not a compile error |
| `cargo test --features mixer` | **BLOCKED — disk** |
| `cargo test --doc` | **BLOCKED — disk** (2 of 6 doctests failed to link) |
| `cargo clippy --all-targets --all-features` | **BLOCKED — disk** |
| `RUSTDOCFLAGS cargo doc --all-features` | **BLOCKED — disk** |
| `cargo build --examples` | **BLOCKED — disk** |

**The blocker, named.** `df -h ~` reads `461G used, 235M avail, 100%` before any build. Every failure above is
`No space left on device (os error 28)` or `ld: write() failed, errno=28` — the trap the skill's machine facts name. The
disk was already at 100 % when this iteration began; reclaiming it means deleting a `target/` or a `/tmp` probe
directory, which the loop's own rules require asking about.

**What that leaves unproven:** the all-features build, clippy, rustdoc and the examples. None of them is touched by this
change in a way `cargo build --features semantic` and the 12 tests do not already exercise, but *unrun is unrun* and
this file says so rather than reporting them green.
