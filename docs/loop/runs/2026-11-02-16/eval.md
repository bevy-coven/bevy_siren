# eval.md — baseline and the `cues.yaml` table, iteration 16

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 80 + 4 doctests green |
| `cargo test --features serde` | 87 + 4 green |
| `cargo test --features mixer` | 87 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 20 GB free. No open issues.

## The `cues.yaml` table

All 30 cases green. Cases 15, 23 and 28 drive a score and a stinger; **neither a case nor any test gives a layer a cue
that is not a loop**.

## Ranked

Probed first, before ranking:

| probe | result |
|---|---|
| `SirenChoices` reset / re-seeded / public `rng` reset | **sound** — the stream reproduces from the seed, and re-writing the same seed continues it (the draw count is the state) |
| a duck whose voice dies | **correct** — a duck is an envelope keyed `(source, target)`, so it recovers on its own clock |
| `SirenSettings::problems` on a duck naming a missing bus | **already reported**, both source and target |
| a layer naming a `Mode::Once` cue | **rank 1**: started, and restarted **every frame** |

1. **A layer naming a cue that ends machine-guns the play path** — `problems()` states the rule, the plugin ignores it,
   and iteration 15's restart rule turned silence into 10 plays in 10 frames. **The pick.**
2. `Refusal::NotALoop` — the honest message for the same fact; public API, decision file
   (`needs-decision/not-a-loop-refusal.md`), written this iteration.
3. Quantise a restarted stem to the bar (iteration 15's item 1).
4. Quantise a retarget to the bar (iteration 14's item 1).
5. A `Refusal` payload that names the cap.

## After the change

| case | after | green? |
|---|---|---|
| a one-shot layer beside a loop layer | only the loop plays; **one** line names the layer and cue in `problems()`' words | green (fails before) |
| 200 frames of a one-shot layer | exactly one line, nothing plays | green (fails before) |
| the cue becomes a loop | the layer starts on the next frame | green (fails before) |
| an unknown cue | still `Refused` with `UnknownCue` | green, unchanged |
| every other case, every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
