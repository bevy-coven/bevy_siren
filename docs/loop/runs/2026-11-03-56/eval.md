# eval.md — iteration 56

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 143 + 6 |
| `cargo test --features serde` | green — 151 + 6 |
| `cargo test --features mixer` | green — 161 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **5.3 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## Three hypotheses tested before this one, and found already correct

The loop's rule is that a pick is an observation, not an impression, so three rank-1 candidates were probed and **passed**:

- **a stolen/stopped voice's duck recovers** — `a_stolen_voices_duck_recovers`: the bus empties, `source_went_quiet`
  rewinds, the duck reaches exactly `0.0`. Green.
- **a bus level changed mid-voice reaches it** — `a_bus_level_changed_mid_voice_reaches_it`: `Music` lands 12 dB down
  within 0.5 s of the table being written. Green.
- **the merge itself** — `a_second_table_leaves_the_first_tables_keys_held`: twenty keys for two ten-key tables.

All three are kept as tests, because they are behaviour a later change could break.

## The item

`SirenSounds` exposes `insert`/`get`/`len`/`keys`/`load_table`/`missing`/`failed` and **no remover at all**; `load_table`
merges. A stale key is never *read* — checked: `missing()`, `failed()` and every `sounds.get` take current-table keys — so
this is a **memory** bug, not a wrong-output one. The cost is the handle: `AudioSource { bytes: Arc<[u8]> }` is the whole
encoded file.

## After

Green: **151 / 159 / 161 + 6**, clippy, fmt, rustdoc, examples. The variant streams and the duck in `render.txt` are
**byte-identical** to iteration 55's. See `review.md`.
