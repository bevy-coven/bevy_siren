# review.md — iteration 40, three passes

## Pass 1 — Function: does it do what the source and the case say?

**The case.** The crate's `Stinger` doc (S1) says a stinger is *"one hit on the music bus, ducking what it plays over"*,
defaulting to −6 dB for 1 s. The example's table carried `music.stinger` and **nothing wrote a `Stinger`** — grep found no
import and no step. So the feature was authored and never shown, and the one duck whose `by` is a mechanism rather than a
cue had never appeared on screen.

**The number, measured on the finished panel** (`evidence/panel/after-95.png`, 1280×720, frame 95):

| What | Row | Log |
|---|---|---|
| the stinger fired | `Music -9.9 dB duck -8.0 dB (newest: stinger at -6.0)` | `0.89s stinger ducked Music by -6.0 dB` |
| its own hit | `3 / 12 voices` on Music | `0.89s played \`music.stinger\` (stinger) on Music at 0.0 dB` |
| the deeper, older duck | — | `0.72s dialogue.greeting ducked Music by -8.0 dB` |

Three places agree: the row's −8.0 is `dialogue.greeting`'s, the row says so, and the −6.0 stinger is named with its own
depth. The stinger's `duck_db` (−6.0) matches S1's documented default exactly.

**The rule that must not break.** `Effects` at the same frame reads `duck -4.0 dB by dialogue.greeting` — one rule, its
own depth, the short form. S2's `by` and S3's `ducked` name the same rule there, so iteration 31's text survives where it
was right.

**Verdict: GREAT.** The feature plays, the row and the log agree, and the number the source reports (−6 dB) is the number
on screen.

## Pass 2 — Zoom: read the plots and crops closely

Read: `evidence/panel/zoom-music-row.png` (crop 1000×60 → 2000×120), `zoom-music-row-1920.png`, `after-95.png`,
`after-95-1920.png`, `fresh-75.png`, `readme-before.png`, and the shipped `docs/media/mixer.png` and `mixer.gif`.

**Layout.** The two-duck clause is 52 characters. I scanned the still for row bands and found **four evenly spaced bands**
(196, 227, 260, 292 — 32 px apart, the row height) with no second band rising from x≈24, so the row does **not** wrap at
1280. At 1920 it has more room, not less. The `Music` row inks to x=817 of 1280.

**No clipping, no hole.** The still's lowest ink sits 11 px above the bottom edge — the same 11 px as `readme-before.png`,
so nothing grew into the margin. The header still reads `7 cues, 2 layers, 4 buses, 5 voices` and the whole of `Problems`
and the four log lines are inside the frame.

**A defect I found and fixed in this pass.** The long form first read `duck -8.0 dB, by stinger -6.0` — correct but
ambiguous ("by −6.0" reads as a preposition of value, not of depth). It now reads
`duck -8.0 dB (newest: stinger at -6.0)`: the parenthesis scopes the second number to the newest rule.

**A second defect, found on the fresh-eyes frame and fixed before shipping.** `Effects` showed
`duck -2.7 dB (newest: dialogue.greeting at -4.0)` while it had **one** duck rule, mid-recovery — the long form invented a
second cause that did not exist. The row now counts **distinct causes**: a rule that is recovering is still that one rule
(`by dialogue.greeting`), and only a *different* `by` makes the depths need separate labels. Both fixes are in
`examples/mixer.rs`'s `Kind::Bus` arm.

**Verdict: GREAT.** After the two fixes: no wrap, no clip, no orphaned number, and each of the three shapes reads as its
own sentence.

## Pass 3 — Fresh eyes: a different seed and a different script

`SIREN_SEED=91 SIREN_KEYS=duck,stinger,steal SIREN_KEY_EVERY=30`, frame 75 (`evidence/panel/fresh-75.png`). Different seed,
different script, different frames.

```
Music    -8.3 dB   duck -6.0 dB by stinger                     3 / 12 voices
Effects  -2.6 dB   duck -2.2 dB by dialogue.greeting           0 / 2 voices
```

The stinger is the **only** duck on `Music` here (the dialogue's released before frame 75), so the plain form is right;
`Effects` is recovering from −4.0 to −2.2, and it too reads plainly. **Nothing needed the long form on this script**, which
is the point: the long form appears only when two rules really overlap, as on the shipped still.

**Can a developer understand it without the code?** Yes, and the panel now answers a question it could not before: *why
is `Music` at −9.9 dB when the log's newest duck says −6.0?* The row itself says `(newest: stinger at -6.0)`, the −8.0
line is visible two entries below, and `music.stinger`'s −6 dB is `Stinger::new`'s documented default, not a mystery
number. The GIF opens at frame 93, **on** the stinger, so the two-duck row is the first thing a reader sees.

**Verdict: GREAT.**

## The shipped picture

`docs/media/mixer.png` and `mixer.gif` are regenerated from this build (still at frame 95 for the two-duck row; GIF from
93, 29 frames, 1280×720). `check_media.py`: **0 problems**. `docs/media/README.md` now records the frame-95 reason and the
new default script.
