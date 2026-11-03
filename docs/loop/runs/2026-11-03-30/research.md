# research.md — iteration 30: the panel's `Problems` block hides the log when there is most to read

## The item

Iteration 26 gave `examples/mixer` a `Problems` block listing `table.problems_in(&settings)` one sentence per line. The
panel is a **fixed column with no scroll**, and the log is the **last** section in it. With six faults the block grows past
the window:

```
1280x720, SIREN_FAULTS=many (six faults)
  ... Problems
  cue `sword.hit.wrong-bus` plays on bus `Reverb`, which the mix table does not have
  cue `ui.click.inverted-pitch` has pitch.low past pitch.high, so no detune can be drawn
  cue `empty.one` has no sounds, so it can only be refused
  cue `empty.two` has no sounds, so it can only be refused
  cue `bus.two` plays on bus `Nope`, which the mix table does not have
  cue `unlimited` sets max_instances 0, ... only the table's max_voices bounds it
  <bottom of the window: the `Log` heading and its ten lines are GONE>
```

The panel's stated job is *"every bus with its level, its duck and its live voice count; the cue table; **the log of what
played and why**"*. A table with several faults — exactly when a person needs the log — pushes it off screen. The view
becomes **least** useful when there is the most to read.

## Sources

**S1. The module doc's own list** — three of the four things the panel promises are above the log; the log is the fourth
and the one a person reads when something is wrong. A layout that drops the last section drops the most important one
first.

**S2. `SirenLog::recent(10)`** — the log block is **already bounded** at ten lines. The crate's own panel bounds one list
and did not bound the other, which is the inconsistency this iteration fixes.

**S3. The skill's step 6, pass 3** — *"can a developer understand the panel without the code?"* A panel whose log is off the
bottom cannot answer *"why is the music quiet"*, which is the question `src/log.rs`'s module doc says the log is for.

**S4. `docs/media/README.md`** — the capture is the crate's evidence; a GIF captured from a state that hides the log would
put a truncated panel in the evidence folder.

## The technique

1. **Bound the block at three sentences**, and when there are more, one line says how many and what to do:
   `and 3 more; the log below is what played`. Three is what fits under the cue list at 1280×720 with the log's ten lines
   still on screen — measured on the capture, the same way the log's ten were chosen.
2. **Plain ASCII.** The first version used an em dash and the panel's font has no glyph for it: the capture showed
   `and 3 more ▯ the log`. A debug view cannot afford a character the font may not have.
3. **The rest is named, not hidden.** The count is on screen, so a person knows the block is truncated rather than complete.

**A `SIREN_FAULTS=many` knob** adds four more deliberate faults to the capture, so the case can be reproduced: two cues
with no sounds, a second cue on a bus the table lacks, and one at `max_instances: 0`. `SIREN_FAULTS=1` (two faults, what the
README's GIF uses) stays under the bound and is unchanged.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the panel promises bus rows, the cue table and **the log** | — | S1 |
| the log block shows ten lines — a bound the panel already had | 10 | S2 |
| the log answers *"why is the music quiet"* | — | S3 |
| the capture is the evidence | — | S4 |

## What good sounds like

A table with six faults shows the first three sentences, `and 3 more; the log below is what played`, then the log. A table
with two — the README's GIF — shows both sentences and no count, because it fits. `Problems / none` on a clean table is
unchanged. No sound changes.
