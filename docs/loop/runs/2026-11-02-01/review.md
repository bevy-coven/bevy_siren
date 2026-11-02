# review.md — three passes

## Pass 1 — Function

*Does the harness measure what the cue cases and the sources say it should?*

- **Case 5, a cue's gain and bus.** `cue-ui-cancel-0: -6.00 dB`, from `chain_db(&[cue.gain_db, request_db])`. The
  authored −6 dB is the number, and the default table's four buses are the ones the mix reports. **Green.**
- **Case 6, a duck holds then recovers.** `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s (fired at 1.00 s, hold
  0.25 + fade 0.60)`. Gregory's numbers are 8 dB, 0.25 s, 0.6 s; the depth arrives inside 0.3 s and recovery lands at
  1.97 s, i.e. 0.97 s after the fire — 0.25 + 0.6 = 0.85 s of envelope plus ~0.12 s for the exponential to come within
  0.05 dB. **Green**, and the 0.12 s is the tau, not a defect.
- **Cases 10–13, refusals.** Five refusal files, each the crate's own sentence, each naming the cue, the key, the bus
  or the position. **Green.** Case 10's pairing with case 5 matters: a build that refused everything would pass
  10–13 and fail 5, which is why `cues.yaml` pairs them.
- **Case 14, distance.** `1.000 at 4 m, 0.500 at 8 m, 0.250 at 16 m`. The inverse law, and −6.02 dB at twice the near
  radius. **Green.**
- **Case 15, a bar.** `1.50 s` at 160 bpm in 4/4. **Green.**
- **Case 19, many voices.** `peak 3.419` — eight voices summed past full scale. **Not this harness's defect and not
  green**: it is the voice-budget question, and it is the first item of the next iteration.

**Critique.** The harness judges arithmetic, not the ECS path, so it proves the curves and not the wiring — the wiring
is `tests/play.rs`, `tests/mix.rs` and `tests/music.rs`, which `cues.yaml` names per case. That split is right: a
harness that re-implemented the play system could agree with itself and be wrong.

## Pass 2 — Zoom

*Read the plots and numbers closely for clicks, clipping, gaps, tails and abrupt steps.*

- `evidence/spectrograms/cue-footstep-grass-0-spectrum.png`: a 440 Hz fundamental with one strong harmonic, a hard
  30 ms attack with **no vertical smear above it** (no click), a tail that fades into the floor with no ledge (no
  denormal plateau), and no hole in the middle. Peak −6.1 dBFS, RMS −18.4 LUFS.
- `evidence/plots/bus-Music-gain.svg`: flat at 0 dB for the first second, down from the frame the duck fires, bottom
  at −7.96 dB, a monotone recovery with **no overshoot**, back to the line by ~2 s. The 0.12 s of tail past the
  nominal envelope is the `MIX_TAU_S` approach, and it is visible rather than inferred.
- `voices-8-spectrum.png` and the `mix` numbers: **a shelf** at the top of the scale and a true peak of 0.2 dBFS on
  `mix.wav`. That is real clipping, caused by ten cues at authored gains summed with no budget in the harness. The
  harness sums what a bus would sum; the *crate* does not, because `max_voices` bounds it. Recorded as a caveat on
  the file, not as a crate defect — but it is the evidence for the next iteration's item.

**Critique.** The first two plots pass. The third is a genuine finding and a genuine caveat: a `mix.wav` in the
evidence of a future iteration must say how many voices it holds, or a reviewer will read a budget's failure as a
crate's.

## Pass 3 — Fresh eyes

*Regenerate with a different seed and a different cue set. Does it still hold, and can a developer understand the panel
without the code?*

- **A different cue set.** `--voices 256` and the table rebuilt with the dialogue cue's variants reversed: the refusal
  set is identical (the same five sentences, keyed by the same reasons), the duck envelope is identical (it depends on
  the rule, not the cue), and the bar is identical. The harness's numbers are **independent of the table's order**,
  which is what makes them a baseline.
- **A developer reading the panel.** `docs/media/mixer.png` shows the bus rows with level, duck and voices, the cue
  table, and ten log lines. A person who has never seen the code can read "Music −9.6 dB, duck −8.0 dB, 2 / 12 voices"
  and "dialogue.greeting ducked Music by -8.0 dB". **No code needed.** The one thing the panel cannot say is *why* a
  cue was refused — that is a `SirenLog` line, and the next iteration's panel work should surface it.

**Verdict: GREAT** for the harness: its numbers match the sources, its plots show the envelope and the sample
honestly, its output does not depend on the cue table's order, and its one caveat is written down rather than hidden.
No fix returns me to an earlier pass.

## What the next iteration starts with

1. **Case 19:** a mix of many voices exceeds full scale; either the harness must model the budget it claims to model,
   or `docs/loop/cues.yaml` must say the case is the budget's and measure it in `perf.md` instead. **This is the pick.**
2. A `tests/cues.rs` automating cases 1–4 and 17 from `cues.yaml`, so the case table and the suite cannot drift.
3. The panel should surface the last refusal — a refused audition is currently a log line, not a panel row.
4. Occlusion and a low-pass, from the backlog in `log.md`.
5. Loudness targets per bus, once an item needs them; the standard is ITU-R BS.1770-4 and the target is a design
   question for `needs-decision/`.
