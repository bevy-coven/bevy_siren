# eval.md — iteration 55

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 144 + 6 |
| `cargo test --features serde` | green — 154 + 6 |
| `cargo test --features mixer` | green — 156 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **4.1 GB free (100 %)** |
| `gh issue list` | no open issues |

**Nothing was red.** `/tmp/pr-target` (1.7 GB) was this loop's own scratch from iteration 54 and was removed, leaving
5.8 GB.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on the `CueProblem` enum decision |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## The item, and the before-numbers

200 cue names driven through the real play path, then the table cut to ten:

| | pickers held |
|---|---|
| after 200 plays | 200 |
| after `reset_histories` | **200** — it clears contents, and is not a retain |
| after the table is cut to 10, one frame | **200**, before the fix; **10** after |

**The test that proves it runs on the old tree**: `the_picker_map_grows_per_cue_name` reads the map through `Debug`, so
it compiles against both. On the old tree it fails at
`assertion left == right failed: a cue the table dropped must not keep its picker — left: 200, right: 10`.

## A regression I introduced and then measured away

The first implementation was `pickers.retain(|name, _| table.get(name).is_some())`. `CueTable::get` is a linear scan, so
that is O(pickers × cues) **per frame**: **989.8 µs at 1000 cues**, 6 % of a 60 Hz frame. Replaced with one pass into a
`HashSet` (**8.5 µs**) and then guarded on `table.is_changed()` (**0 ns** on a static table). Recorded in `perf.md`
because the number is the reason for the shape.

## After

Green: **147 / 155 / 157 + 6**, clippy, fmt, rustdoc, examples. Three new tests. The variant streams in
`evidence/after/render.txt` are **byte-identical** to iteration 54's — the fix moved no choice, which is the requirement.
See `review.md`.
