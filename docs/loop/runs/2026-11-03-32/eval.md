# eval.md — baseline and the `cues.yaml` table, iteration 32

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 113 + 5 doctests green |
| `cargo test --features serde` | 120 + 5 green |
| `cargo test --features mixer` | 122 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 %). Iteration 31's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 15/23 drive music; **no case and
no capture asks what the panel shows about the score.**

## The lead, and what it became

Iteration 31's lead: the panel lists `track.calm` and `track.combat` as ordinary cues and draws no `MusicState`. Confirmed —
`draw_panel` takes four resources and none is music, while the example ships a two-layer score at 160 bpm whose `Layer` step
really crossfades on the bar.

Probing the **height budget** first, because iteration 30's lesson is that the column is fixed:

| measurement | value |
|---|---|
| content at the shipped layout | ~892 px in a 720 px window — **already over** |
| lowest ink in the README's capture (before) | **y = 719** of 720 — **1 px margin, a clipped glyph** |
| free height after showing eight log lines | **12 px** |
| a four-line Music block needs | ~134 px |

So the panel's log **was already being clipped**, in every capture, and the crate had documented the budget: `CAPACITY`'s
doc says *"a panel shows eight"* while the example asked for ten.

## Ranked

1. **The log is clipped: the panel asks for ten lines where the crate documents eight and the column fits 8.5** — rank 1 for
   the panel, found by looking, and it unblocks the next item by freeing height. **The pick.**
2. **A Music block** — `MusicState`'s five panel-facing accessors are drawn by nothing; needs ~134 px against 12 free, so it
   needs a layout decision first. **Measured, deferred.**
3. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
4. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| the log's last line | ends on its eighth line; lowest ink `y = 708`, 12 px margin | green (`y = 719`, clipped, before) |
| lines shown | 8 — the documented budget | green |
| every other section | unchanged | green |
| every test, both feature sets | unchanged (113 / 120 / 122 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
| free height for the next item | **12 px** measured, ~134 px needed | recorded |
