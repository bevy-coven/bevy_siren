# research.md — iteration 32: the panel's log was clipped, and the crate already said how many lines fit

## The item

The panel's log arm asked for `log.recent(10)`. The column is fixed and full, so the **tenth line was cut off mid-glyph** —
visible in every capture, including the README's: the last row read `1.10s played engine.idle` with the bottom half of the
next line's letters sliced away.

`SirenLog::CAPACITY`'s own doc already names the budget:

> *"How many entries the buffer keeps. **A panel shows eight**; a test reads one; the rest is for a person asking what just
> happened."*

So the crate documented eight, the example asked for ten, and the window fit about 8.5.

Measured after the change, from the capture's own pixels: the lowest ink is at **y = 708** of 720 — 12 px of margin, no
clipped glyph.

## Sources

**S1. `SirenLog::CAPACITY`'s doc** — *"A panel shows eight"*. The number is in the crate, not invented here; the example
simply used a different one.

**S2. Iteration 30's trap** — a panel is a **fixed column with no scroll**: every section added takes height from the ones
below, and the last one loses. That iteration bounded the `Problems` block; this one found the same defect one block down,
in the **log**, where the loss is subtler — a sliced glyph rather than a missing section.

**S3. The skill's step 6** — *"Read every image with `read`"*. The clip is invisible in the numbers (`recent(10)` returns
ten entries) and obvious in the picture. It survived iterations 29–31, which each regenerated this image.

**S4. `SirenLog::line()`** — one sentence per entry, no wrapping contract. The panel joins entries with `\n` and Bevy wraps
each `Text` at the container width; a line that does not fit the column is **drawn anyway** and clipped by the window, not
elided.

## The technique

`recent(8)` instead of `recent(10)` — the documented budget, and what the column fits.

**What this does not do, and why.** The visible item this iteration wanted was a **Music block**: the example ships a
two-layer score at 160 bpm, the `Layer` step really crossfades `calm → combat` on the bar, and `MusicState` exposes
`layer_db`, `target_db`, `names`, `moving` and `next_change_s` — **none drawn**. But the column has **12 px** of free height
measured from the capture, and a four-line block needs ~134 px. Showing music means the cue list gives up rows, which is a
layout decision with its own evidence; it is the next item, with the height budget measured rather than guessed.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a panel shows eight log lines | 8 | S1 |
| a fixed column: every section takes height from those below | — | S2 |
| the clip is visible only in the picture, not in the numbers | — | S3 |
| the log joins sentences with `\n`; a line that does not fit is clipped, not elided | — | S4 |
| free height at eight log lines, measured from the capture | **12 px** | measured |

## What good sounds like

A panel whose log ends on its eighth line with the bottom margin intact — no sliced glyphs — and which shows exactly the
number of lines the crate documents. No sound changes.
