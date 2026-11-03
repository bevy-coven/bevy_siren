# review.md — iteration 8, three passes

## Pass 1 — function

**The claim.** A voice is placed — panned, given a `Transform`, attenuated — if and only if `cue.spatial` **and** the
play has a position. Before, `spawn_voice` decided the panner from `request.position.is_some()` alone.

**Against the sources.** `Cue::spatial`'s doc defines what the flag does both ways except one: a non-spatial cue given a
position. The code gave such a voice to `rodio`'s two-ear panner, which applies `min(1, 1/d²)` as well as a pan —
**−38.7 dB at 8 m** by the crate's own `backend_spatial_gain` — while `Played.gain_db` reported **0.00 dB**. The
crate's guide states the principle that condemns it: *"playing it flat, or at the origin, is a different sound from the
one the game asked for."*

| request | before | after |
|---|---|---|
| non-spatial cue, `.at(8 m)` | panned, −38.7 dB audible, `Played.gain_db = 0.00` | flat at its own gain; `Played.gain_db` is true; no `Transform` |
| spatial cue, `.at(8 m)` | panned, −6.02 dB, compensated | unchanged (test 2) |
| spatial cue, no position | flat | unchanged (test 3) |
| spatial cue, no listener | `SpatiallyMute` | unchanged (test 4) |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**One predicate, three places.** `answer` now computes `spatial` once and `Started` carries it, so the attenuation, the
panner flag and the `Transform` cannot disagree again. The two "must not regress" tests are the ones that would catch
an overshoot — a build that stopped placing *spatial* voices would be green on the new test and red on those.

**Verdict: GREAT.** Wrong output, silent, on the path every build has, and the crate's own report was the lie.

## Pass 2 — zoom

**Read:** `evidence/{before,after}/render.txt`, the cue spectrograms, and the four new tests' output.

**A defect the gate found, and fixed:** both `PlaybackSettings` assertions panicked under `--features mixer`, because
that build's own graph owns playback and there is no settings component at all. They are now asserted only on Bevy's
path (`if !cfg!(feature = "mixer")`), which is where the panner is; `Played.gain_db` and the `Transform` shape are
asserted on both. This is the crate's own rule — *the two backends answer identically* — applied to a test that had
quietly only tested one.

**A trap confirmed rather than discovered:** `render.txt`'s new panner line reads the same in both trees, because it
computes the panner's law rather than the routing decision. It is evidence of the **magnitude** (0.0117 linear,
−38.7 dB) and not of the fix; the fix's evidence is `tests/play.rs::a_non_spatial_cue_ignores_a_position`, which fails
on the unfixed tree at "a non-spatial cue is not routed through the panner". Written into `perf.md` so the next reader
does not mistake identity for proof.

**Audio is unchanged**, which is correct: 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS / −12.1 dBFS looped past the
gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS, the spectrum decaying to −120 dBFS with no vertical edge.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The four tests are a matrix over the two booleans (`cue.spatial`, `request.position`) plus
the listener: placed/flat/refused, each pinned. The `model` test pins the panner's own number so the magnitude cannot
drift. The probe that found the bug ran the two mismatched rows and printed the gains.

**Can a developer understand it without the code?** The `CHANGELOG` says what changed and who could notice: a project
that relied on the accident loses a pan it was never promised, and its meter was wrong while it happened.
`render.txt` states the panner's law in dB beside the cue's.

**What a fresh reader would still ask:** should a non-spatial cue given a position be *refused* rather than ignored?
That is a real question and it costs an exhaustive `match` for every consumer, so
`needs-decision/non-spatial-position.md` carries it with the sentence it would print and a recommendation.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a test that tested one backend); fixed.
