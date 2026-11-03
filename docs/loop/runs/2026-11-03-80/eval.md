# Iteration 80 — eval

## The item: a regression I shipped in iteration 77 and never saw

`cargo test` had not run for **five consecutive iterations** — the disk blocked it — so a failing test sat in `main`
since `63ba157`:

```
library::the_semantic_feature_declares_no_dependency_of_its_own ... FAILED
  left:  ["dep:ort", "dep:rodio", "dep:rubato", "dep:ebur128"]
  right: ["dep:ort", "dep:rodio", "dep:rubato"]
```

Iteration 77 added `ebur128` to the `semantic` feature and to `ALLOWED_DEPS` in `tests/leaf.rs`, but the **second**
ratchet — `tests/library.rs`, which pins the feature's value string — was updated in neither. Two tests guard the same
decision from opposite ends, and I updated one.

**This is the exact cost of the disk.** Not a wrong decision, an unseen one: the guard that would have caught it was
the suite that could not link.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| 1–33 (audio behaviours) | **green** | `cargo test` **198 passed, 0 failed**; `--features serde` **207**; `--features mixer` **208** |
| the ratchet | **green, was red** | both `tests/leaf.rs` and `tests/library.rs` now name `ebur128`, each with its argument |
| 19 (perf) | **green, first time in five iterations** | 0.084 / 0.472 / 1.576 µs/frame at 8 / 64 / 256 — see `perf.md` |

## Baseline — complete, first time since iteration 75

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |
| `cargo test --lib --features semantic` | **pass** — 20 |

## The disk, actually solved rather than worked around

Six iterations were blocked by `errno=28`. The fix was **not** dependency cleaning, which is what dug the hole:

    $ cargo clean -p bevy_siren --target-dir target
         Removed 645 files, 6.3GiB total

The crate's **own** `target/debug/examples` held **4.2 GB** of stale binaries, and `deps` another 2 GB of its own
artifacts. Cleaning the crate (not its dependencies) took the volume from **122 MB** to **6.2 GB** free in one command,
and every step above then ran. `~/.cache/huggingface` remains 50 GB of FLUX checkpoints, untouched and still the
larger prize should it ever be needed.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74 and 75 — which is the check that the audio path was not touched.
