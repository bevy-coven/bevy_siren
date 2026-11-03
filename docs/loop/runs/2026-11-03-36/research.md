# research.md — iteration 36: the panel shows the score, and the height it costs

## The item

`examples/mixer` ships a **two-layer score at 160 bpm**, adds `MusicPlugin`, and its `Layer` step really crossfades
`calm → combat` on the bar. `MusicState` exposes **seven** panel-facing accessors — `layer_db`, `target_db`, `names`,
`moving`, `live`, `next_change_s`, `voice` — and the panel drew **none**. The cue list showed `track.calm` and `track.combat`
as ordinary loops, so the music transition the script performs was invisible in the picture.

The block was deferred **four times** (32, 33, 34, 35) for one reason, measured each time: the column is fixed, unscrollable,
and has **12 px** of free height against ~134 needed if the block is a heading plus two rows.

## Sources

**S1. `MusicState`'s seven accessors** — `next_change_s`'s own doc says *"for a test or a panel that says 'in 0.8 s'"*;
`moving()` and `target_db()` exist to describe a crossfade. A method written for this panel, drawn by nothing, for the fourth
time.

**S2. `docs/guide/music.md`** — *"Music: layers that change on the bar"*, and the example writes `.next_bar().fade_s(2.4)`.
The panel is where a person judges a crossfade; if it cannot show one, the guide's claim is untestable in the editor.

**S3. Iteration 32's measurement** — a fixed column is a **budget**: a heading is 30 px plus a 10 px gap, a row is 18–21 px
plus the same gap. The three music cues in the list were **63 px**; a heading plus one line is **68 px**. The block costs
**5 px net**, and the margin was 12.

**S4. `SirenLog::CAPACITY`'s doc** — *"a panel shows eight"*. That sentence describes **a** panel, not a constant: `CAPACITY`
is 256. Seven lines is within its spirit and is what the measured column fits with one more block above the log.

## The technique

1. **Move the layer rows out of `Cues` and into a `Music` block.** The block lists the score's layers from
   `MusicState::names()`, each with `layer_db` and — while `moving()` — an arrow to `target_db`. The cue list filters out any
   cue the score owns (`music.score.layers[].cue`), so a music cue appears **once**, where it can show live gain.
2. **One line for the whole score.** A row per layer cost a full line more than the three cue rows it displaced — captured:
   margin 12 px → **1 px**, a clipped glyph again. Two layers fit on one line in **41 of ~114 columns**, so the block is a
   heading plus a line and the log keeps its space.
3. **Seven log lines, not eight.** Measured: the block costs 68 px against the 63 px freed, so eight lines leaves **1 px**;
   seven leaves **11 px**. `CAPACITY`'s "eight" describes a panel without this block.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| accessors written for a panel | 7 | S1 |
| a heading costs 30 px + 10 px gap; a row 18–21 px + gap | — | S3 |
| three cue rows freed | 63 px | S3 |
| heading + one line | 68 px | S3 |
| log lines that fit with the block | **7** | measured |
| margin at seven | **11 px** | measured |

## What good sounds like

A panel whose `Music` block reads `calm -4.0 dB   combat -80.0 dB` at rest and `calm -79.9 -> -80.0   combat -3.1 -> -3.0`
mid-crossfade — the score's own transition, in the crate's own numbers — with the seven cue rows below it carrying only
cues that are **not** layers, and the log ending cleanly. No sound changes.
