# review.md — iteration 36, three passes

## Pass 1 — function

**The claim.** `MusicState` exposes seven panel-facing accessors and the panel drew none, while the example ships a
two-layer score whose `Layer` step really crossfades on the bar. Deferred four times for height; this iteration made the
layout decision.

| state | source | before | after |
|---|---|---|---|
| the score at rest | S1 (`MusicState`'s accessors), S2 (the guide: *"layers that change on the bar"*) | `track.calm` and `track.combat` as ordinary cue rows | `Music / calm -4.0 dB   combat -80.0 dB` |
| the score **mid-crossfade** | S1 (`target_db`, `moving`) | invisible — the cue row showed `loop +0.0 dB` | `calm -79.9 -> -80.0   combat -3.1 -> -3.0` |
| after the bar | S1 | invisible | `calm -80.0 dB   combat -3.0 dB` |
| the cue list | S3 (the column is a budget) | nine rows, three of them layers | **seven** rows, no layer among them; `music.stinger` stays (a one-shot, not a layer) |
| the log | S4 (`CAPACITY`: *"a panel shows eight"*) | eight lines, margin 12 px | **seven** lines, margin **11 px** |

**Against the sources.** S2 is what the block is for: the guide's title is *"layers that change on the bar"*, and a panel that
cannot show a change happening cannot let anyone judge that claim. S4 is why the log loses a line: `CAPACITY`'s "eight"
describes a panel **without this block**, and the measurement is the authority.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{music-block-1280,zoom-final-music,final-mid-fade}.png`, the regenerated
`docs/media/mixer.{png,gif}`, and pixel scans of the before/after stills.

**A defect I introduced and then measured away.** My first version was a row per layer. The block then cost more than the
three cue rows it displaced: **margin 12 px → 1 px**, a clipped glyph — iteration 32's defect, reintroduced. Two fixes,
each measured: the score on **one line** (41 of ~114 columns), and **seven** log lines instead of eight. Final margin
**11 px**.

**A wrong reading, corrected by looking again.** I first thought `combat -80.0 dB` was a bug; it is `MusicLayer::new`'s
documented default — a layer starts silent and gameplay brings it in. And my early mid-fade captures at frames 112–175 were
**before the bar**: the example runs at ~8.3 ms/frame (≈120 fps), so 1.5 s is ~180 frames, not 90. The capture at 195 shows
the fade.

**Verdict: GREAT** after the two measurements.

## Pass 3 — fresh eyes

**A different shape.** Four captures of one binary at four frames — rest, mid-fade (195), after the bar (230), and the
fault-table GIF — plus a pixel scan of the still. The mid-fade frame is the one that matters: a **static** row cannot show
`-> target`, and that arrow is the whole reason for the block.

**Can a developer understand the panel without the code?** Yes: five blocks now read as one story — buses with their named
ducks, the score with its layers and targets, the cues that are not layers, the report, and the log. Before, the music
appeared twice (as cue rows) and never showed a transition.

**What a fresh reader would still ask:** *why seven log lines?* Because a heading plus a line costs 68 px and the three
cue rows freed 63; the arithmetic is in `research.md` and the margin is measured, not guessed. `CAPACITY`'s "eight" is a
description of a panel, and this panel has one more block than that sentence knew about.

**Verdict: GREAT.**
