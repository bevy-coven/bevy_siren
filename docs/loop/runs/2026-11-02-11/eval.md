# eval.md — baseline and the `cues.yaml` table, iteration 11

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 66 + 4 doctests green |
| `cargo test --features serde` | 73 + 4 green |
| `cargo test --features mixer` | 73 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 14 GB free. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 29 cases green. **Case 9 is the one this iteration looks at**: it says an unstealable voice "is never stolen; the
request is refused", and its observation names the **voice**. The suite proves the rule through
`the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue.

## The pick, and what the pass actually found

Ten iterations have closed every rank-1 defect the suite and the probes could find, and the five next-5 items are
public-enum decisions for the owner. So this pass tested six candidate defects and **found none**:

| candidate | measurement |
|---|---|
| `DuckClock.started` grows with table swaps | 50 fires over two rules leave **two** entries — keyed by rule, not by fire; stale pairs are never read |
| a stop on the play's own frame cuts instead of fading | `update_mix` writes `Applied` before `stop_requests`; the voice fades for 0.48 s |
| `Played.gain_db` disagrees with the audible chain | −2 −3 −6 = **−11.00 dB**, and `Applied` reads −11.0 |
| a stealable request can take an unstealable voice | refused; nothing stopped |
| a bus cap above the table's raises the budget | clamped with `.min(total)` |
| `SirenLog` drops the wrong entry at capacity | keeps 256, drops the oldest |

What is left is legibility: the crate's strongest promise has one test, and its path takes a paragraph to explain. Two
tests now state it directly, and the mutation shows they and the old test all catch the same break.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical (no `src/` change) | green |
| **a stealable request cannot take an unstealable voice** | `VoiceLimit`, live count 1, the voice still there, no `stolen by` line | green |
| **a stealable voice still plays beside one** | two live voices, nothing stolen | green |
| **the mutation** | removing `!c.unstealable` reddens both new tests **and** the table-budget test | measured |
