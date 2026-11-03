# eval.md — baseline and the `cues.yaml` table, iteration 26

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 109 + 5 doctests green |
| `cargo test --features serde` | 116 + 5 green |
| `cargo test --features mixer` | 117 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 14 GB free (98 %). No open issues. Iteration 25's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 18 runs `problems_with`;
**no case and no capture has ever shown a panel drawing a bad table**.

## The lead, and what it became

Iteration 25's log recorded, verified: `draw_panel`'s four resources are `SirenMix`, `CueTable`, `SirenSettings`,
`SirenLog` — **`problems()` is not among them**. Confirmed by reading the cue arm: it formats the cue's own fields and
nothing else. So a cue on a bus the mix lacks, or with an inverted pitch range, renders exactly like a healthy one.

## Ranked

1. **The panel draws a broken cue as a healthy one** — the picture that exists to be judged says the cue is fine.
   **The pick**, unblocked.
2. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
3. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. `Refusal::NotALoop` (`needs-decision/not-a-loop-refusal.md`).

**Items 2–5 remain blocked on one public-enum decision** (asked in iteration 24; not re-asked).

## After the change

| case | after | green? |
|---|---|---|
| clean table | `Problems` / `none`, no row marked | green (no section before) |
| table with a bad bus and an inverted pitch | two sentences in the block; both rows `<- problem` | green (drawn as healthy before) |
| `problems()` vs `problems_in` on the example's table | `problems_in` prints `none`; `problems()` would print **five false `UnknownBus`** | green — iteration 21's fix is load-bearing |
| every existing row, every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
