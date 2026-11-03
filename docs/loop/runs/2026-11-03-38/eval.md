# eval.md — baseline and the `cues.yaml` table, iteration 38

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 119 + 5 doctests green |
| `cargo test --features serde` | 126 + 5 green |
| `cargo test --features mixer` | 128 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 11 GB free (98 %). Iteration 37's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. **No case reads the panel's
header** — it is a string in an example.

## The lead, and what it became

Iteration 37's lead was the two images' table difference. Looking at the still shipped by iteration 36 found something
closer: the header said **`9 cues`** and the cue list beneath it showed **seven**. Iteration 36 moved the score's two layers
into a `Music` block and left the header counting `table.cues.len()`.

The panel already had both numbers; it printed one.

## Ranked

1. **The header counts the table, not the panel** — a number the reader can check and find wrong. **The pick.**
2. The still/GIF table difference — **prose**, both images correct, recorded.
3. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
4. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| the clean table's header | `7 cues, 2 layers, 4 buses, 4 voices` | green (`9 cues`, wrong) |
| cue rows beneath it | 7 | green |
| the fault table's header | `9 cues, 2 layers` over **9** rows | green (`9 cues`, by coincidence) |
| a table with fewer cues than layers | `(0, 2)`, no underflow | green (probed) |
| every test, both feature sets | unchanged (119 / 126 / 128 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
| the still's margin | 11 px | green |
