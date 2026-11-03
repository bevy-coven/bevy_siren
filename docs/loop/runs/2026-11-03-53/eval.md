# eval.md — iteration 53

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 139 + 6 |
| `cargo test --features serde` | green — 147 + 6 |
| `cargo test --features mixer` | green — 150 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **3.3 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.** Disk arrived at 100 %; `/tmp/pz-target` (1.7 GB) was this loop's own scratch from iteration 52 and
was removed, leaving 4.9 GB. `target/` (38 GB) is not mine and was not touched.

## A mistake that damaged the repository, and was caught

A `sed -i '' 's|^\[dependencies\]|…|' Cargo.toml` in a **scratch crate** matched the `[dependencies]` table of
`bevy_siren`'s own `Cargo.toml` through the path dependency, inserting `ron` and `serde` into the crate. `git diff` showed
it immediately and it was reverted in the same step. The lesson: never edit a manifest with a pattern that can also match
a resolved dependency's manifest.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## The item

`MusicScore` derived `Default` (`beats_per_bar: 0.0`) while the field carried
`serde(default = "MusicScore::default_beats_per_bar")` (**4.0**). Measured on the same score:

| path | `beats_per_bar` | `bar_s()` | `problems()` |
|---|---|---|---|
| RON, key omitted | 4.0 | `Some(2.0)` | `[]` |
| `MusicScore::default()` | **0.0** | **`None`** | **`["beats_per_bar is not a positive number"]`** |

The crate's own default produced a score its own report called broken, and with no beat grid every `NextBar` change
landed immediately.

## After

Green: **141 / 149 / 151 + 6**, clippy, fmt, rustdoc, examples. Four new tests; `a_default_score_quantises_to_the_bar`
**fails on the old tree** (`assertion left == right` at the bar) and passes on the new. See `review.md`.
