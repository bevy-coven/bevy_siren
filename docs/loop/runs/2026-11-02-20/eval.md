# eval.md — baseline and the `cues.yaml` table, iteration 20

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 92 + 4 doctests green |
| `cargo test --features serde` | 99 + 4 green |
| `cargo test --features mixer` | 100 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 16 GB free. No open issues. Iteration 19's run has its `review.md`.

## The `cues.yaml` table

All 30 cases green. Case 18 runs `problems_with` on a cue table; **no case runs `SirenSettings::problems()` on a
non-default bus list**, which is the gap.

## The lead, and what it became

Iteration 19 ended with a probe, not a guess, and the probe found **two** faults:

| table | before |
|---|---|
| `buses: []` (documented: the crate's four) | **4 sentences**, all false — `Dialogue`, `Music`, `Effects`, `Dialogue` "which the table does not have" |
| two buses (a menu mix) | `Dialogue` reported **twice** |
| one bus + two rules naming `Ghost` | `Ghost` reported **twice** |
| default table | 0 (correct) |

The first is the worse one: `docs/guide/buses.md` says an empty list means the crate's four *"everywhere — for a play's
bus check, for the panel, and for the mix"*, and `bus_names()` implements it. `problems()` built its own set from
`self.buses` — the identical bypass iteration 10 closed in `update_mix`.

## Ranked

1. **The report invents four faults for a correctly mixed table, and names one fault twice** — a report that cries wolf
   teaches an author to ignore the panel. **The pick.**
2. `Refusal::NotALoop` (iteration 16's item 1; decision file waiting).
3. `CueProblem::BadGain` + the refusal for a bad request (iteration 18's decision file).
4. Re-time a restarted stem to the bar (iteration 15's item 1).
5. Quantise a retarget to the bar (iteration 14's item 1).

## After the change

| table | after | green? |
|---|---|---|
| `buses: []` | **0** sentences about buses | green (fails before) |
| two buses | **1** sentence for `Dialogue` | green (fails before) |
| one bus + two rules naming `Ghost` | **1**, with the exact words | green (fails before) |
| default table | 0 | green before and after (the guard) |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
