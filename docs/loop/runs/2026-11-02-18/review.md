# review.md — iteration 18, three passes

## Pass 1 — function

**The claim.** The crate had **two opposite answers to one caller mistake**. `docs/guide/spatial.md` states the policy
plainly — *"a caller handing in a `NaN` transform gets a sound at the listener's feet, not a `NaN` volume that silences
the whole bus"* — and `distance_gain` implements it. `chain_db` did the opposite for a gain: a non-finite part returned
`SILENT_DB`.

| row | source | before | after |
|---|---|---|---|
| a cue authored `gain_db: NaN` | S1 (spatial's stated policy), S2 (R6: *"a play is never silent"*) | `Played gain -80.00` — **silent**, reported as an authored level | `gain 0.00`; and the sound is at the level the cue would have had |
| `Play::gain(NaN)` | S2 | `gain -80.00`, silent | `gain 0.00` |
| `Play::gain(-∞)` | S2 | `gain -80.00`, silent | `gain 0.00` |
| `Play::pitch(NaN)` | S3 (`semitones_to_ratio` already plays `1.0×`) | `pitch NaN` — the meter disagrees with the device | `pitch 0.00`, which *is* `1.0×` |
| `Play::pitch(1e9)` | S3 (the clamp is four octaves each way) | `pitch 1000000000.00` for a sound at `16×` | `pitch 48.00` |
| a bus at `SILENT_DB` (guard) | S4/S5 (the model's silence protects the mix) | silent | **unchanged** — silent |

**Against the sources.** S3 is the sharpest: `semitones_to_ratio` **already** neutralised a non-finite detune to `1.0×`
and clamped a large one to `16×`, so the **sound** was always right; only the number `Played` reported was wrong. R6's
second half — *"everything a play decides is readable"* — is exactly about that: a meter that reports a value the device
is not at is worse than no meter.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack}.png`, the loudness table, both probe files.

**Audio unchanged, deliberately.** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS, `footstep-grass`
−18.4 LUFS / −6.1 dBFS (looped past the gating block), `mix.wav` −4.3 LUFS / +0.2 dBFS — identical to iterations 14–17.
This is a **number** fix: no rendered cue was authored with a `NaN`, so a changed WAV would mean the change had reached
further than it should. The attack spectrogram is unchanged: full-band onset, clean cut at 0.033 s.

**The measured table**, `evidence/nan{,-before}.txt`, seven rows:

```
before                                         after
plain                 gain   0.00 pitch   0.41   unchanged
cue gain NaN          gain -80.00 pitch   0.41   gain   0.00
play gain NaN         gain -80.00 pitch   0.41   gain   0.00
play gain -inf        gain -80.00 pitch   0.41   gain   0.00
play pitch NaN        gain   0.00 pitch    NaN   pitch   0.00
play pitch 1e9        gain   0.00 pitch  1e9.00 pitch  48.00
```

**A judgement recorded, not hidden.** I considered refusing a non-finite request instead of neutralising it, and wrote
both into `needs-decision/bad-gain-problem.md`: a new `CueProblem` variant and a new `Refusal` variant are **breaking**
for exhaustive `match`es, and refusing would *lose* a sound that used to play. Neutralising is what the crate already
does for a distance. The stricter answer is the owner's call, with its cost stated.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The tests enter by three doors — a cue's own gain, a slot's gain (via the same `finite_or`), a
`Play` override, and a detune — and the pitch test exercises **two** plays in one frame (a `NaN` and a `1e9`) so the
normalisation and the clamp are shown side by side. The guard test builds a table with **one** bus at `SILENT_DB`, a
shape no other play test uses, because the guard must be a real bus and not a default.

**Can a developer understand it without the code?** `docs/guide/buses.md` gains *"A number that is not a number"*: the
one rule, the two places it applies (distance, gain/detune), why refusal was not chosen, and where the decision file
is. `Played`'s own doc says *"the gain it started at"* — the fix is what makes that sentence true.

**What a fresh reader would still ask:** *why report `48` rather than `1e9`?* Because `1e9` semitones is not a rate any
device plays; `48` is the rate it does play, and the meter's job is the device.

**Verdict: GREAT.**
