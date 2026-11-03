# plan.md — iteration 50: the master is the headroom trim, and two demonstration files stop clipping

## The pick, and why it beats the runner-up

**Pick:** iteration 49 left backlog #3 as *"an art decision"* — the shipped table clips. It is **not** an art decision:
`needs-decision/above-unity-gain.md`'s recommendation (C, do nothing) rests on *"`SirenSettings::master_db` is the tool for
it"*, and nothing tested that sentence or said it in the guide. This iteration measures it, applies it to both
demonstration files, and states it where a reader looks.

Beats the runner-up (a `MissingSound` capture): that is a picture; this is **two files written past full scale** and a
claim the crate never checked.

## The blocked item

**Case 31** — still blocked on `needs-decision/bad-gain-problem.md`. Nothing built from it, and the loudness question is
**not** blocked: `above-unity-gain.md` already recommends C, so a report was never the answer.

## The design

1. **`tests/mix.rs::the_master_is_inside_the_worst_peak`** — `worst_peak_db` reads `0`, `−6`, `−20` dBFS for one unity
   voice at those masters. S1's claim, measured for the first time.
2. **`examples/mixer`**: `Effects` `0.0 → −7.0`, `Music` `−2.0 → −7.1`. `2 × 10^(−7/20) = −0.98 dBFS`; at −7.1, `−1.08`.
   Both buses a decibel inside the rail, and the panel's peak clause disappears because nothing reaches 0 dBFS.
3. **`examples/render`'s `mix.wav`** written at `settings.master_db` instead of `sum_db(&gains)`, with the report line
   saying so. The line previously applied **+18.58 dB** to a 0.5 tone — `4.2×` full scale.
4. **The guide**: *"`master_db` is the trim, and it is inside the number"*, and why `problems()` stays silent.

## Fields

**No new field, no new API.** The iteration uses `master_db` and `worst_peak_db`, both existing.

## Tests

- **`the_master_is_inside_the_worst_peak`** — three masters, three readings, each within 1e-3.
- Everything else is proved by the harnesses' own numbers: the panel's peak clause vanishing, and `ebur128` on the new
  `mix.wav` (`−6.1 dBFS` true peak against `+0.2` before).

## The carve

- **Not** a limiter, an auto-trim or a `problems()` sentence for loudness: `above-unity-gain.md` recommends C and says
  why — a boost is a mixing decision, and reporting it cries wolf on every stem lifted above unity.
- **Not** a change to `master_db`'s `0.0` default: a crate default that quietly attenuates a project's whole mix is a
  behaviour change needing its own decision file, and the guide now says what to turn.
- **Not** the render harness's `voices-8/64/256` files: `voices-8` is deliberately 8 sources summed to exercise clipping,
  and its line already says the budget is a sample cost (iteration 49).
