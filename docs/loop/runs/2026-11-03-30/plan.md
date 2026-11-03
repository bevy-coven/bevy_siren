# plan.md — iteration 30: the panel's `Problems` block is bounded so the log survives

## The pick, and why it beats the runner-up

**Pick:** iteration 26's `Problems` block grows without limit in a **fixed, unscrollable column**, and it sits **above** the
log. With six faults the log — the section that answers *"why is the music quiet"* — is pushed off the bottom of the screen.
Measured on a 1280×720 capture: the `Log` heading and its ten lines are not there. The panel is least useful exactly when
there is most to read.

Beats the runner-up (a `Stopped` line for a parent's despawn): that one is blocked on the public-enum decision six items
already wait on. This one is unblocked, it is **rank 1 for the panel** — a view that drops its most important section — and
it is provable by looking.

## The design

1. **`SHOWN = 3`** sentences, then `and N more; the log below is what played` when there are more. Three is what fits at
   1280×720 with the log's ten lines still on screen — measured, not guessed.
2. **Plain ASCII:** the first version's em dash drew a box (`and 3 more ▯ the log`); the font has no glyph for it.
3. **`SIREN_FAULTS=many`** adds four more faults to the capture so the case reproduces; `SIREN_FAULTS=1` is unchanged
   because two sentences fit under the bound.

## Fields

None. No `src/` change, no public type, no schema; test counts unchanged (112 / 119 / 121 + 5).

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **A capture at six faults**: three sentences, the count, and the `Log` heading **on screen** with its lines. Before:
  the log is absent.
- **A capture at two faults**: both sentences, **no** count, log present — the bound does not fire when the list fits.
- **A capture of the clean table**: `Problems / none`, unchanged.
- **All three looked at**, plus a zoomed crop of the block.
- `check_media.py` after regenerating the README images: every reference ok, 0 problems.

## The carve

- **Not** a scrollable panel: a Bevy UI `ScrollPosition` on the column is a different layout, and a capture cannot show
  the part below the fold — the evidence would be worse than a bound.
- **Not** a `Problems` heading move below the log: the crate's own order is header, buses, cues, verdict, log, and the log
  last is where a person looks; the **block** is what must not grow.
