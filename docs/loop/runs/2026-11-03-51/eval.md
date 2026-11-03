# eval.md — iteration 51

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 133 + 6 |
| `cargo test --features serde` | green — 141 + 6 |
| `cargo test --features mixer` | green — 144 + 6 |
| `cargo test --doc --all-features` | green — 6 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **5.8 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — **blocked** on `needs-decision/bad-gain-problem.md` |
| 32 | a despawned owner stops its sound | green | iteration 43 |
| 33 | a refused layer is virtual and says so | green | iteration 44 |

## A stale backlog entry, corrected

Iteration 50's top-5 listed *"`Duck::new`'s zero hold/fade defaults"* as remaining. **It was already shipped**:
`SirenSettings::problems()` reports it (`src/mix.rs:217`) and `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time`
exists at line 958. I wrote a duplicate test, found the original, and deleted mine.

## The item

Not on any list, found by probing: **`SirenChoices::pick` fixes each cue's `VariantPicker` capacity at first play**, and
`remember` bounded the window by the deque's *allocator* capacity. Measured with four keys and seed 11:

| picker | `avoid` asked | 24 picks |
|---|---|---|
| `new(1)` | 1 | `adbcbcabcacabcabcbabcbcb` |
| `new(1)` | **3** | `adbcbcabcacabcabcbabcbcb` — **byte-identical** |
| `new(3)` | 3 | `adbcadbcadbcadbcadbcadbc` |

## After

Green: **134 / 141 / 144 + 6**, clippy, fmt, rustdoc, examples. Six new tests, and
`tests/play.rs::a_cue_whose_avoid_grows_gets_the_wider_window` **fails on the old code** (`d, c, b, a, c, d` — a key
returning after two picks at `avoid: 3`) and passes on the new. See `review.md`.
