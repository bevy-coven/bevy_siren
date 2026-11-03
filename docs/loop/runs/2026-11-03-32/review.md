# review.md — iteration 32, three passes

## Pass 1 — function

**The claim.** The panel asked `SirenLog::recent(10)` in a fixed column that fits about 8.5, so the **last log line was
clipped mid-glyph** in every capture — including the README's own still. `SirenLog::CAPACITY`'s doc says *"a panel shows
eight"*; the example asked for ten.

| row | source | before | after |
|---|---|---|---|
| the log's last line | S1 (`CAPACITY`: *"a panel shows eight"*), S4 (`line()` joins with `\n`; a line that does not fit is **clipped**, not elided) | **clipped mid-glyph**; lowest ink at **y = 719** of 720 — 1 px of margin | ends on its eighth line; lowest ink at **y = 708** — **12 px** of margin |
| the number of lines the panel shows | S1 | 10 | **8** — the documented budget |
| every other section | S2 | unchanged | unchanged |
| every sound | — | — | **byte-identical** (21/21) |

**Against the sources.** S1 is the whole argument: the crate had documented the budget and the example used a different
number. S4 explains why the failure was a **slice** rather than a wrap — `line()` returns one sentence, the panel joins with
`\n`, and Bevy draws a `Text` that does not fit and lets the window cut it.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{eight-log-1280,readme-before}.png` and the regenerated `docs/media/mixer.{png,gif}`.

**The measurement is from the picture, not from arithmetic.** A script reads the lowest row with ink in each capture:
**before `y=719` (1 px margin, the slice is at the frame edge), after `y=708` (12 px)**. My earlier arithmetic
(~892 px of content in 720) was in the right direction and the wrong magnitude; the pixels settled it.

**A scope decision, recorded rather than taken quietly.** The item I set out to build was a **Music block** — the example
ships a two-layer score, the `Layer` step really crossfades `calm → combat` on the bar, and `MusicState`'s five panel-facing
accessors are drawn by nothing. But the same pixel measurement says the column has **12 px** free at eight lines, and the
block needs ~134. Shipping a clipped log **and** a block that pushes it off screen would be two defects for one. The
budget is in `eval.md` so the next run starts from a measurement.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The check is not a test but a **pixel measurement on the capture**, which is the only instrument that
can see a clipped glyph — `recent(10)` returns ten entries whether or not the window shows them. Two captures, one before
and one after, same binary, same script.

**Can a developer understand the panel without the code?** Yes, and now the log is readable to its last line. Before, the
panel's evidence — the thing `src/log.rs`'s module doc calls *"the last decisions … for a person asking what just
happened"* — ended in a sliced glyph.

**What a fresh reader would still ask:** *why eight and not nine?* Because `CAPACITY`'s doc says eight, and the capture has
12 px of margin at eight. Nine would fit by ~14 px and leave nothing for a wrapped `Problems` line; eight is the documented
number and the measured one.

**Verdict: GREAT.**
