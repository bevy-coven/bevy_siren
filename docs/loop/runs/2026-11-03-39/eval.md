# eval.md — baseline and the `cues.yaml` table, iteration 39

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 119 + 5 doctests green |
| `cargo test --features serde` | 126 + 5 green |
| `cargo test --features mixer` | 128 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 9.8 GB free (98 % — the tightest this loop has recorded; a release build wants ~2 GB, so a release
would need a clean-up first). Iteration 38's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Cases 10–13 assert refusals through
`Refused` and `SirenLog` — **none asks whether a panel draws one.**

## The lead, and what it became

Iteration 38's log left it: *"the panel has never shown a refusal"*. Confirmed by reading the script: `steal`, `duck` and
`layer` **all play**; nothing refuses. `Entry::line` already writes the sentence; nothing produced one.

Two measurements followed:

| measurement | value |
|---|---|
| the fault table's cost in log lines | **3** (two cue rows + two problem sentences) |
| fault table at 7 log lines | margin **1 px — clipped** |
| fault table at 5 | margin **1 px** |
| fault table at 4 | margin **11 px** |
| the refusal's visibility window | ~30 frames |

## Ranked

1. **The panel had never shown a refusal** — the message the skill calls the most valuable, undrawn. **The pick.**
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. A gain above 0 dB (`needs-decision/above-unity-gain.md`; recommendation to not add it).

## After the change

| case | after | green? |
|---|---|---|
| the still's log | opens with `refused \`no.such.cue\`: no cue of that name` | green (never refused) |
| the default script | `steal,layer,duck,refuse` | green |
| the fault table's log | **4** lines, margin **11 px** | green (7 lines, 1 px, clipped) |
| the clean still | 7 lines, margin 11 px | green |
| the GIF | native 1280, every block legible; first frame at 88, **before** the refusal | green (960 wide) |
| every test, both feature sets | unchanged (119 / 126 / 128 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
