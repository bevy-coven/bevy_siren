# Iteration 108 — eval

## Baseline before anything

Nothing red: `cargo test` 199 + 6 doc; `--features semantic` 109 lib + **436** + 6 doc; `--all-features --test all` **460**;
clippy, fmt and docs clean; `cargo build --examples --features semantic` ok.

## After the change

| run | result |
|---|---|
| `cargo test` | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 doc (the only build that compiles `tests/table.rs` without `semantic`) |
| `cargo test --features mixer` | 209 + 6 doc |
| `cargo test --features semantic` | 109 lib + **436** + 6 doc |
| `cargo test --all-features --test all` | **460** |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |
| `cargo build`, `cargo build --examples --features semantic` | ok |
| `git status -- src` | **no change** — `src/cue.rs` returned to its original content hash after the experiments |

The test count did not move: the iteration **extends** `problems_names_each_mistake`; it adds no test.

## The cue cases

All 33 of `cues.yaml` green, unchanged. This iteration touches a **test**, not behaviour. One edit to the table: case 18's
`authored` listed four of the seven mistakes (`no_sounds`, `duplicate`, `unknown_bus`, `inverted_pitch`) — a fourth copy of
the set, already stale — so it now **names the one list** (`kinds!` in `tests/table.rs`) instead of restating it, the repair
iteration 106 made to a doc. The file still parses (33 cases).

## The cases

| Case | Green/red | Observation |
|---|---|---|
| every `CueProblem` has a fixture cue **in this test** | **RED before** | seven variants, six rows; `InertAvoidRepeat` (iteration 105) was produced by no cue here — it **is** tested separately (`tests/play.rs`, `tests/model.rs`), so the hole is narrower than "untested": this test's job, *every problem prints as a sentence naming its cue*, never reached it |
| a variant added to the enum | **compile error**, already was | `E0004 … &CueProblem::Tenth { .. } not covered`. **This is not new**: `HEAD`'s `named` closure listed every variant, and the guard fired in iteration 105 |
| a variant added **and listed**, no fixture | **RED before, green-catching now** | `HEAD` passes (that is how 105 got through); the new test fails: `no fixture cue produces ["Tenth"]` |
| the `named` closure's claim | **false** | its comment said it did not enumerate the variants; it enumerated all seven |
| the `named` closure's assertion | **vacuous** | implied by `problems.contains(&problem)` one line above, which compares the name too |
| 436 semantic / 460 all-features | **green** | nothing else moved |

## A correction to this iteration's own first design

I first wrote *"no guard exists"* and added a second exhaustive `match`; adding a variant failed to compile, and I counted
that as proof. **It proved nothing**: `HEAD` fails to compile in exactly the same way, because its closure was already
exhaustive. The control I had not run was in the record already — `needs-decision/diagnostic-enums-are-extensible.md`
quotes the iteration-105 `E0004` at `tests/table.rs:66`. The real defect was that the compiler's demand is satisfied by an
**arm**, and a variant needs a **fixture**. Found by reading `git diff` instead of trusting the experiment; the research
file records it in its own section.

## The item

`tests/table.rs::problems_names_each_mistake`: a `kinds!` macro lists every `CueProblem` variant once and generates the
exhaustive `match` and the count from it; the fixture gains the `inert` cue and a row; the test fails if any listed kind has
no fixture. No change to `src/`.
