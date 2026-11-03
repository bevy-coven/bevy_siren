# review.md — iteration 37, three passes

## Pass 1 — function

**The claim.** `SIREN_SHOT_DIR` sampled every `SIREN_SHOT_EVERY`th **update**. A layer fade is `MIX_TAU_S` = 50 ms, which at
~8.3 ms/update is **6 updates**; the shipped GIF sampled every **12**. **0.5 samples per fade** — the `-> target` arrow that
iteration 36 built the `Music` block for appeared by luck of the phase.

| row | source | before | after |
|---|---|---|---|
| samples per 50 ms fade | S1 (`MIX_TAU_S`), S2 (the knob's doc: *"every …th frame"*) | **0.5** (12 updates × 8.3 ms) | at `EVERY=4` (40 ms): **1.25**, and the capture kept 47 frames |
| frames carrying the arrow | S3 (the arrow is the block's reason) | one frame, if the phase fell right | **11 consecutive** (`f_0028`–`f_0038`) |
| the knob's unit | S4 (no magic constant) | updates, converted by a literal `8.333` in my first fix | **hundredths of a second** — a duration for a duration |

**Against the sources.** S4 is the one that changed the design: my first version turned updates into milliseconds with a
hard-coded `8.333`, which is exactly the magic constant the crate forbids. The knob is now a duration, and the conversion
disappears. S3 is why it matters: a showcase whose feature appears by chance is not evidence.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{fade-frame-31,zoom-fade-31}.png`, the regenerated `docs/media/mixer.gif`, and a scan of all 47
captured frames for the music line's ink width.

**The proof is a count and a crop.** The scan reports the music line's ink width per frame; `f_0028`–`f_0038` are **wider**
than the settled line by 61–114 units — the arrow's extra glyphs — and `zoom-fade-31.png` reads
`calm -76.2 -> -80.0   combat -6.9 -> -3.0`. Eleven consecutive frames where there used to be half a frame.

**The old GIF, inspected rather than assumed:** 20 frames, and the music line's ink changed only across 12–16. It **did**
catch the fade — by luck of the phase. A showcase that is right by accident is one refactor from being wrong.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The check is a **per-frame pixel scan of the capture directory**, not a test: the property is
temporal, and no assertion in `tests/` can see a GIF. The scan counts frames whose line is wider than the settled one,
which is exactly "did the arrow render".

**Can a developer understand it without the code?** The knob's doc now says *"hundredths of a second"* and **why** — *"it is
a duration because what the GIF has to catch is a fade"*. A person writing the capture command no longer has to know the
machine's frame rate.

**What a fresh reader would still ask:** *why not keep the default at 12?* Because 12 hundredths is **120 ms**, twenty times
the sampling the old 12 *updates* usually gave — the number changed meaning when the unit did, and the default is now `2`
(20 ms), four times denser than a 50 ms fade.

**Verdict: GREAT.**
