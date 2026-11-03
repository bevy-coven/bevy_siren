# review.md — iteration 4, three passes

## Pass 1 — function

**The claim.** `MusicLayers::fade_s` is the time a layer takes to arrive; before this iteration every move eased at
`MIX_TAU_S` (~150 ms) whatever the project wrote.

**Against the sources.** The field's own doc says *"How long the move takes once it starts"*, and the crate's meaning
of a fade duration everywhere else is **the time to arrive**: `Duck::fade_s` plus `active_duck_db` reaches the target
at exactly `hold + fade`. So the fix is `tau = fade_s / ln(1000)` — the constant for which `approach` is within one
part in a thousand at `fade_s` — derived in `research.md` §"The technique" and pinned as
`tests/model.rs::a_fade_arrives_within_one_part_in_a_thousand` (which asserts arrival at `fade_s` **and** that the move
is not already finished at `fade_s / 2`).

| measurement | before | after |
|---|---|---|
| an authored 0.8 s fade, read at 0.3 s | **−9.13 dB** (settled) | **−13.6 dB** (still arriving) |
| the same fade at 0.85 s | −9.0 dB | −9.07 dB → its target |
| `fade_s(0)` at 0.5 s | −9.0 dB | −9.0 dB (the default easing is unchanged) |
| a layer not named by a change | — | keeps its own fade (test 3) |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**A second defect the tests found, and fixed in the crate rather than the test.** `approach` returns its target when
`dt <= 0` — right for a bus that must be at level on a game's first frame, wrong for a layer: the frame that applies a
change can be that frame, and then the whole authored fade is spent in one step. `move_layers` now **holds the gain**
on a frame with no elapsed time. Without this, the fix would have been correct in the tests that `advance()` first and
wrong in the one frame a real game hits first — the kind of gap that leaves a bug in place while the suite is green.

**Verdict: GREAT.** One derivation, one call site, one extra private field, four tests, and a defect found while
testing that would have survived a fix written to the test.

## Pass 2 — zoom

Read: `plots/layer-fade.svg`, `plots/layer-fade-zoom.svg`, `plots/stinger-envelope.svg`,
`spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack,tail}.png`, and the emitted polylines.

**Three defects in my own plot, found and fixed:**

1. **The vertical scale was a fixed `0 dB…lo`.** A fade from silence drawn against 0 dB is a line pinned to the floor,
   and a fade to −9 dB wasted two thirds of the frame. The scale is now the data's own (`−71.5 … −2.9 dB` here).
2. **The marker label read "duck fires" on a plot with no duck.** `write_gain_plot` now takes the label.
3. **A marker at t = 0 sat on the y axis** and its label collided with the axis's own labels. A marker is drawn only
   when the moment is inside the plot.

**The curves.** Blue (`fade_s(0.8)`) is still rising where red (`MIX_TAU_S`) has flat-lined at −9 dB by ~0.4 s; blue
reaches the line at ~0.8 s and the crate's 0.01 dB snap finishes it. The gap between them at 0.3 s is 4.5 dB — visible
as the vertical distance between the curves at the second gridline.

**Loudness, unchanged:** a cue reads −24.4 LUFS / −12.1 dBFS (looped past the 0.4 s gating block), `mix.wav` −4.3 LUFS /
+0.2 dBFS, both identical to iterations 1–3 because no cue gain moved.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The tests run three scores and three arrangements: a per-layer fade (`calm` long,
`combat` default), a zero fade, and a layer the change does not name. The derivation test sweeps four durations
(0.2, 0.3, 0.8, 2.0 s) at 240 fps, so it is the arithmetic and not one lucky number.

**Can a developer understand it without the code?** The plot answers *what changed* — two labelled curves, an axis in
dB and seconds, a caption naming the authored duration — and the harness prints the same fact as a sentence:
*"0.3 s in, the authored 0.8 s fade reads −13.6 dB and the default easing reads −9.1 dB"*. `render.txt` is readable by
a person who has not opened `src/`.

**What a fresh reader would still ask, and the honest answer:** what happens if a **second** change arrives while the
first is still fading? Today it replaces both the target and the fade — the layer does not queue — and that is now
written down in `plan.md`'s Risk section and in the log's next-5 rather than left to be discovered. Queueing is a
feature with its own semantics (does the second change start from the current gain, and does it wait for the first?),
not a bug fix.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 1 (the rule) or pass 2 (a plot label); neither is
outstanding.
