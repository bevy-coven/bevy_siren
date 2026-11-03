# eval.md — iteration 54

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 143 + 6 |
| `cargo test --features serde` | green — 151 + 6 |
| `cargo test --features mixer` | green — 153 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **6.8 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.** `/tmp/pq-target` (2.2 GB) was this loop's own scratch from iteration 53 and was removed, leaving
8.9 GB.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## The item, and the before-numbers

Iteration 53 logged `DuckClock::started` as *"real but unobservable — no accessor, so no capture could prove it"*.
That was wrong: `DuckClock` derives `Debug`, so the map is readable through `{:?}` without any API change.

| scenario | before |
|---|---|
| 2000 `fire`s, then `prune` at t=1000 | **2000 entries still held** |
| 2 fires against one target, `prune` at t=1000 | the dropped rule **still held** |
| 100 table edits, each firing its own rule, `prune` at t=1000 | **100 entries** |

Reachable: `DuckClock::fire` is public, and the crate's own path grows whenever a table's rule set changes — which the
Cauldron does on every edit of `family/bevy_siren.ron`.

## After

Green: **144 / 152 / 154 + 6**, clippy, fmt, rustdoc, examples. Three new tests plus `rule_starts()`. A running duck is
unchanged across a retain (`−8.0 dB`, verified in `evidence/plots/bus-music-gain.svg`), and the new call costs
**13 ns per frame** measured directly. See `review.md`.
