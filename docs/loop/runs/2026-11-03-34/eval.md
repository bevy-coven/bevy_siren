# eval.md — baseline and the `cues.yaml` table, iteration 34

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 116 + 5 doctests green |
| `cargo test --features serde` | 123 + 5 green |
| `cargo test --features mixer` | 125 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 12 GB free (98 % — watched, since a release build wants ~2 GB). Iteration 33's run has its
`review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 15/23 drive a score; **no case
puts a layer's cue on a bus other than `Music`.**

## The lead, and what it became

Iteration 33's log named the probe: *"a layer naming a cue on a non-`Music` bus (`problems` says a layer "should name the
Music bus" — does the plugin refuse, or mix it on Effects while the score says music?)"*. Probed:

| | before |
|---|---|
| `MusicScore::problems` for such a score | **`[]`** — nothing reported |
| the layer's voice | **`wrong.bus on Effects`** — it started there |
| `MusicState::live()` | **2** — the mirror says two layers |
| `SirenMix` voices | Effects **1**, Music 1 |

So the score and the sound disagreed, and the crate's own report was silent. The stinger path already refuses this exact
fault (`play_stingers`); the layer path had neither the check nor the line.

## Ranked

1. **A music layer plays on another bus and nothing says so** — the score and the sound disagreeing, rank 1, silent.
   **The pick.**
2. **A Music block in the panel** — rank 4, deferred a third time for height (12 px free against ~134). The budget is
   measured and unchanged.
3. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
4. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| a layer naming a loop on `Effects` | not started; `live() = 1`; report **and** log name layer, cue, bus | green (started, `live() = 2`, no report) |
| `problems()` for that score | one sentence | green (`[]` before) |
| a score of two `Music` layers (guard) | both start; `problems()` empty | green |
| every test, both feature sets | 118 / 125 / 127 + 5 | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
