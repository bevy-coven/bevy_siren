# eval.md — baseline and the `cues.yaml` table, iteration 36

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 119 + 5 doctests green |
| `cargo test --features serde` | 126 + 5 green |
| `cargo test --features mixer` | 128 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 12 GB free (98 %). Iteration 35's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Cases 15 and 23 drive a score and
assert on `MusicState`'s numbers; **no case asks what a panel shows.**

## The lead, and the decision this iteration finally made

The **Music block** was deferred four times (32, 33, 34, 35) for one reason, measured each time: the column is fixed and
unscrollable, and the block needed more height than was free.

The measurements, taken in this order:

| step | result |
|---|---|
| free height before the block | **12 px** |
| a heading costs | 30 px + 10 px gap |
| three music cue rows freed | 63 px |
| heading + **one** line | **68 px** |
| after a row-per-layer block | margin **1 px — clipped glyph** |
| after one line + seven log lines | margin **11 px** |

So the decision is: **one line for the score, seven log lines.** A row per layer reintroduced iteration 32's clipped
glyph, measured rather than assumed.

## Ranked

1. **`MusicState`'s seven accessors drawn by nothing** — the crate's headline feature invisible in its own showcase.
   **The pick**, deferred four times and finally built.
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. A gain above 0 dB (`needs-decision/above-unity-gain.md`; recommendation to not add it).

## After the change

| case | after | green? |
|---|---|---|
| the score at rest | `Music / calm -4.0 dB   combat -80.0 dB` | green (absent before) |
| mid-crossfade | `calm -79.9 -> -80.0   combat -3.1 -> -3.0` | green |
| after the bar | `calm -80.0 dB   combat -3.0 dB` | green |
| the cue list | seven rows, no layer among them | green (nine, three of them layers) |
| the log | seven lines, margin **11 px** | green (eight, margin 12) |
| every test, both feature sets | unchanged (119 / 126 / 128 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
