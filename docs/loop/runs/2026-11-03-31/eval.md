# eval.md — baseline and the `cues.yaml` table, iteration 31

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 112 + 5 doctests green |
| `cargo test --features serde` | 119 + 5 green |
| `cargo test --features mixer` | 121 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 %). Iteration 30's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 6 measures the duck at attack,
hold and release — as **numbers**, never as **what the panel shows**.

## The lead, and what it became

Iteration 30's lead asked two questions of the panel: does it show a duck that ended, and is the header's count the one a
person needs? Reading the code answered both:

| finding | verdict |
|---|---|
| `SirenMix::deepest_duck` — *"for a panel that says what a line of dialogue did"* — is **never drawn** by the panel it was written for | **the pick** |
| the header counts cues, buses and voices; a duck count would repeat the rows | no item |

Probed further, the bus row shows `duck -8.0 dB` and never **by what**, while `Entry::Ducked` records `by`. A person must
find the log line to learn which cue lowered the bus.

## Ranked

1. **The bus row says how far, not by what** — a missing fact on the row a person is reading, with the answer already in
   the panel's hand. **The pick.**
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| Music ducked by a line | `duck -8.0 dB by dialogue.greeting`, one line at 1280 and 1920 | green (name was absent) |
| the duck releases | clause disappears; `duck +0.00 -> row shows no duck` | green (probed) |
| a second duck on the same bus | the newest `Ducked` wins | green (test) |
| a bus nothing ducked | no name | green (test) |
| every test, both feature sets | 113 / 120 / 122 + 5 | green (+1 on each from the new test) |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
