# plan.md — iteration 38: the header counts what the panel shows

## The pick, and why it beats the runner-up

**Pick:** the header says *"9 cues"* and the cue list shows **7**. Iteration 36 moved the score's layers into a `Music` block
and left the header counting `table.cues.len()`, so the one number a reader can check by counting rows is wrong. The panel
already had the two numbers — it just printed one.

Beats the runner-up (the still/GIF table difference, iteration 37's lead): that one is **prose** — the reader is told the two
images use different tables — and both images are correct as shipped. This one is a number on screen that the rows beneath
it contradict.

## The design

`Kind::Header` names the split: `{cues} cues, {layers} layers, {buses} buses, {voices} voices`, with
`cues = table.cues.len().saturating_sub(layers)` and `layers` from `music.score`.

- **`saturating_sub`**: a layer may name a cue the table lacks; `problems()` reports that, and the header must not underflow.
- **Four numbers, four blocks**: cues → the `Cues` rows, layers → the `Music` block, buses → the `Buses` rows, voices → the
  sum of the bus rows' live counts. Every one checkable.

## Fields

None. Example only; no `src/` change, no public type, no schema.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **The still, looked at:** `7 cues, 2 layers` over seven cue rows and a two-layer block; margin unchanged at **11 px**.
- **The fault-table capture, looked at:** `9 cues, 2 layers` over **nine** rows — 7 shipped + 2 faults — so the header tracks
  the table it is given.
- **The arithmetic's edges, probed:** `9,2 -> (7,2)`, `2,2 -> (0,2)`, `1,2 -> (0,2)` with no underflow, `0,0 -> (0,0)`.
- **A layer naming a missing cue** is reported in `problems()` — the case `saturating_sub` exists for.
- `check_media.py`: 0 problems after regenerating both images.

## The carve

- **Not** a "9 cues (2 layers)" phrasing: it re-invents the sum the reader would have to compute, and the split is what the
  panel actually shows.
- **Not** a test for the header: the example is outside the suite, and the header is a string with no logic worth pinning
  beyond the `saturating_sub` probed above.
