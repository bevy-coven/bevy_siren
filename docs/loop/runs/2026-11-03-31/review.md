# review.md — iteration 31, three passes

## Pass 1 — function

**The claim.** The bus row shows **how far** a bus is ducked and never **by what**, while `Entry::Ducked` records the cue and
`SirenMix::deepest_duck`'s doc says it exists *"for a panel that says what a line of dialogue did"*.

| state | source | before | after |
|---|---|---|---|
| Music ducked by a line | S1 (`Ducked`'s `by` field), S3 (R6: everything a play decides is readable) | `Music -9.4 dB duck -8.0 dB` | `Music -9.5 dB duck -8.0 dB **by dialogue.greeting**` |
| the same bus after the duck releases | S2 | — | **no duck clause at all** — measured: `after: duck +0.00 -> row shows no duck` |
| a second duck on the same bus | S1 | — | the **newest** `Ducked` wins (pinned in `tests/mix.rs`) |
| a bus nothing ducked | S1 | — | no name shown (`by("Ambience") == None`) |

**Against the sources.** S2 is the sharpest: a method whose doc names this exact panel, unused by it. The crate recorded
the fact, the panel had the record in hand, and the row showed two of three fields.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{duck-by-1280,duck-by-1920,no-duck-1280}.png`, the regenerated `docs/media/mixer.{png,gif}`, and
the probe's two lines.

**It fits, and the measurement said it would.** The row is **71 characters** with the annotation against roughly **114
columns** of 18-px monospace in the 1232-px content box. The 1280 capture shows `Music -9.5 dB duck -8.0 dB by
dialogue.greeting 2 / 12 voices` on one line, with the bus column aligned beneath it — iteration 30's wrap defect does not
recur.

**The claim is checked against the log in the same picture.** The row says `by dialogue.greeting` at `-8.0 dB`; the log two
blocks down says `1.57s dialogue.greeting ducked Music by -8.0 dB`. Two places, one claim — which is what a panel is for.

**A deliberate choice, recorded:** I did **not** draw `deepest_duck` in the header. A header that repeats the deepest of the
rows beneath it is noise, and the method stays available to a host. The carve is written in `plan.md`.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** Three states: ducked at 1280, ducked at 1920, and the **released** state — the last reached through a
probe rather than a capture, because the scripted run re-fires a duck every 20 frames and I could not find a frame without
one. That is the honest way to test the boundary: drive the state, not the animation.

**Can a developer understand the panel without the code?** Yes: the row now answers the question a developer has while
watching — *what is making the music quiet?* — without correlating two blocks.

**What a fresh reader would still ask:** *why the newest entry and not the deepest?* Because the row already shows the
**depth** (`-8.0 dB`, the mix's own number, which is the deepest of all active rules); the annotation's job is only to name
the cue, and the newest one is the most recent action. If two rules duck the same bus, the depth is the deeper's and the name
is the later firing — both true, from two different sources, which the test pins.

**Verdict: GREAT.**
