# review.md — iteration 2, three passes

## Pass 1 — function

**The claim.** On the `mixer` backend, a `Mode::Once` voice's slot in the voice budget is released when its sample
ends; before this iteration it never was.

**Against the source.** Gregory §14.5.8.4 (lines 23093–23129 of the home-still copy): *"a sound engine typically
supports only a fixed number of simultaneous voices … so we must use them wisely"*, and a virtual voice "muted or
stopped temporarily … **frees its resources for use by other voices**". A voice that has produced its last sample and
keeps its slot is exactly the resource leak the source says the engine must not have.

**Against the case table.** No `cues.yaml` case covered it — that is why the table could not see it, and the table
gains a case this iteration. What the table *did* have: case 8 (a bus cap holds) and case 9 (an unstealable voice is
refused). This iteration's evidence is the second half of each:

| scenario | before | after | expected |
|---|---|---|---|
| 40 one-shots, cap 32, priority 0 | 40 played, peak **40** live, **0** ended, last frame **32** live | 40 played, peak **3** live, **40** ended, last frame **0** live | a voice ends with its sample; the count returns to zero (`after/steady/budget.txt`) |
| an *unstealable* once-cue (priority 180), cap 4, 6 plays | **4** played, **2** `VoiceLimit`, **0** ended, 4 live forever | **6** played, **0** refused, **6** ended | the lockout the first row hides: with a stealable cue the full bus steals instead, so the count hides the leak |
| a music layer moved to −3 dB | mixer voice never written; layer stayed at its start gain | `Applied { db: -3.0 }`, the number the mixer's voice stage reads | `MusicState::layer_db` and the mix agree (`tests/mixer.rs`) |

**What the second row taught the review.** The first "before" run looked *almost* fine — every play was answered
`Played` — because with a stealable cue the budget was permanently full and each play stole an audible voice. That is
worse than a refusal and completely silent: 32 footsteps, then every later footstep cuts off one already sounding.
The lockout row is what makes the defect visible to a reader.

**Verdict: GREAT.** The failure is fixed at its cause (`retire_voices` asks the backend the trait already declares),
the second bug the test found (`update_mix` never wrote a mixer voice's gain) is fixed in the same place by the same
rule, and both have a test that fails on the unfixed tree.

## Pass 2 — zoom

Read, in order: `plots/before-steady.png`, `plots/after-steady.png`, `plots/before-lockout.png`,
`plots/after-lockout.png`, and the four cue crops in `spectrograms/`.

**The plots.** Four defects were found and fixed before this pass could be judged:

1. **The plot first had no legend** — two lines and a caption, and a reader had to know which was which. Fixed: a
   swatch and a label per line, and axis labels (`0`, the axis top, `frame →`, `cap N`).
2. **The caption reported the axis top as the peak.** The axis is scaled to include the cap so a run that never
   reached it still reads against it; the caption said "peak 32" for a run whose peak was 3. Fixed: the caption
   reports the run's own peak and the axis is its own number with 12 % headroom.
3. **The cap line sat exactly on the axis top**, so its label collided with the axis's. Fixed by the headroom above.
4. **The first rasterisation was blank**, because `var(--c1)` is a CSS variable the rasteriser does not resolve. The
   SVG is now committed as the artifact (the reader's own renderer resolves the theme colours), and the PNG in
   `plots/` is a substituted-colour copy for a reviewer who cannot open an SVG.

What the final plots say: the before-steady count rises 1 → 40 and stays pinned at the 32 cap; the after-steady count
is a low band that returns to zero. The before-lockout count rises to the cap in four steps and stays there with the
refusal line stepping up beside it; the after-lockout count never reaches the cap.

**The cue crops.** `ui-cancel-spectrum.png` shows the synthesised cue's attack inside the first 30 ms and a decay to
−80 dB by 0.31 s, with no vertical line and no gap; `ui-cancel-attack.png` resolves that attack into a clean onset
across the band and no click at 0.035 s. Both are **byte-identical to iteration 1's**, which is the point: this
iteration changed no number the renderer drives.

**A number that looks like a defect and is not.** `mix.wav` reads −4.3 LUFS at +0.2 dBFS true peak. That is the same
as iteration 1, and it is the harness summing ten cues at their authored gains without a budget; it is case 19's
open question, not a regression. A single cue reads −24.4 LUFS at −12.1 dBFS (looped past the 0.4 s gating block),
also unchanged.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**Does it still hold with a different shape of run?** Regenerated with a different voice count, cap, priority and
sample life than the recorded pair: `--voices 6 --cap 4 --priority 180` is the lockout pair, and the steady pair is
40/32/0; the harness was run at both and at `--voices 240 --cap 256` while checking the 256-voice column. The
retirement rule holds in each: an ended once-voice is gone the next frame, a loop is not, a fading voice is not
retired twice, and the counts in `SirenMix::voices` follow.

**Can a developer read it without the code?** The `budget.txt` summary answers the four questions a person asks —
how many plays, how many refusals, what the peak was, and how many `ended` lines — in eight lines, and the plot's
legend and caption are in `docs/loop` vocabulary ("live voices", "VoiceLimit refusals", "cap"), not in the crate's.

**What a fresh reader would still ask:** why a `VoiceLimit` refusal names the *bus* when the cap that was hit was the
table's (`Refusal::VoiceLimit { bus: Some(bus) }` in `answer`, even when `room_for` failed on the table budget). That
is a real mislabel and a separate item — recorded in the log's next-5, not fixed here, because it changes the
`Refusal` message a panel prints and wants its own decision file if the shape changes.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 1 (the rule), or pass 2 (a plot caption), neither
of which is outstanding.
