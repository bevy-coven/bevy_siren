# plan.md — iteration 32: the log shows the eight lines the crate documents

## The pick, and why it beats the runner-up

**Pick:** the panel asked `SirenLog::recent(10)` in a fixed column that fits about 8.5, so the **last log line was clipped
mid-glyph** in every capture — including the README's own still. `CAPACITY`'s doc says *"a panel shows eight"*; the example
used ten.

Beats the runner-up (**a Music block** — the example ships a two-layer score, the `Layer` step really crossfades on the bar,
and `MusicState`'s five panel-facing accessors are drawn by nothing): that one is the **bigger** feature, but the column
has **12 px** of free height measured from the capture and the block needs ~134, so building it needs a layout decision
first. Shipping a clipped log *and* a music block that pushes the log off screen would be worse than either.

## The design

`recent(8)`. That is the whole change: the documented budget, and what the column fits.

## Fields

None. No `src/` change, no public type, no schema.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **The capture, looked at**: the log ends on its eighth line, lowest ink at **y = 708** of 720 — 12 px of margin, no
  clipped glyph. Before: a sliced line below the tenth.
- **The measurement**: free height read from the capture's pixels, not from an estimate.
- **The README images regenerated**, because the panel's log changed; `check_media.py` confirms every reference.
- Full gate; counts unchanged (113 / 120 / 122 + 5) — no `src/` line moved.

## The carve

- **Not** the Music block. The column is full; a four-line block needs ~134 px against 12 px free. It is the next item, and
  `eval.md` records the height budget so the next run starts from a measurement.
- **Not** a scrollable column: a capture cannot show what is below the fold, so the evidence would be worse than a budget.
- **Not** tighter spacing: the panel's 10 px row gap is what makes it readable at 1280; trading legibility for one more log
  line is the wrong way round.
