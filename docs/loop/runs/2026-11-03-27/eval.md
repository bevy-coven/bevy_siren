# eval.md — baseline and the `cues.yaml` table, iteration 27

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 109 + 5 doctests green |
| `cargo test --features serde` | 116 + 5 green |
| `cargo test --features mixer` | 117 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 14 GB free (98 %). No open issues. Iteration 26's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 6 measures the duck at its
**attack, hold and release** — with a voice that keeps playing throughout. **No case stops the source and asks what the
duck does.**

## The lead, and what it became

Iteration 26's lead was a missing `SirenLog` `Recovered` entry. Probed, the **state** is already readable
(`SirenMix::ducked` returns `0.0`), so that is a log gap and a public-enum decision — written down as
`needs-decision/duck-recovers-entry.md`, not built.

The probe that paid was next to it. A duck is documented as a rule about a **voice**; `DuckClock` keys on
`(source_bus, target_bus) -> started` and never learns the voice stopped. Stopped on frame 0, the duck held for 15 more
frames and then faded:

| case | before |
|---|---|
| the line is stopped on frame 0 | `+0.0` at frame **60** — 0.25 s hold + 0.6 s fade for a sound already gone |
| a loop keeps playing | `+0.0` at 60 (correct) |
| a second line at frame 20 | extended (correct) |

## Ranked

1. **A duck outlives the voice that justified it** — rank 1, and the skill names *"a duck that does not recover"*
   explicitly. **The pick.**
2. `SirenLog::Recovered` — decision file written this iteration; blocked on the public-enum question.
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| the line is stopped on frame 0 | recovering by frame 10, `+0.0` at **frame 40** | green (was 60) |
| a loop keeps playing | full hold, `+0.0` at 60 | green (guard, unchanged) |
| a second line at frame 20 | extended, not zero by 70 | green (guard, unchanged) |
| a one-shot that ends by itself | `+0.0` at 60 | green (guard, unchanged) |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
