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

All green, after reclaiming disk mid-run (see the note below).

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 passed, 0 failed |
| `cargo test --features serde` | **pass** — 207 passed, 0 failed |
| `cargo test --features mixer` | **pass** — 208 passed, 0 failed |
| `cargo test --doc` | **pass** — 6 passed, 0 failed |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |
| `cargo test --features readme-example --test all readme::` | **pass** — 1 passed |
| `cargo test --lib --features semantic` | **pass** — 12 passed |

**The disk, recorded because it shaped the run.** The volume was at **100 %, 122–251 MB free** when the iteration
started, and the first six all-features steps failed with `ld: write() failed, errno=28` and
`No space left on device (os error 28)` — the exact trap the skill's machine facts name, and *not* a code error. With
permission, the two prior iterations' `/tmp` probe directories (2.9 GB) were deleted and `cargo clean -p` was run on the
heaviest dependency artifacts (`bevy_pbr`, `bevy_render`, `bevy_ui`, `bevy_sprite_render`, `bevy_ecs`, `wesl`,
`bevy_asset`) as each step needed room. Every step above then passed. **Nothing was deleted that this iteration did not
create or receive explicit permission to remove**, and no `target/` was deleted wholesale.

**One real defect clippy found** that the compiler did not: the reference literals in the new test carried ten
significant digits for `f32` values (`clippy::excessive_precision`). Truncated to the eight `f32` actually holds; the
measurements they encode are unchanged and the test still passes at the same 0.06 dB bound.
