# review.md — iteration 19, three passes

## Pass 1 — function

**The claim.** Iteration 18 gave the crate one rule for a non-finite caller value and applied it to the five numbers
that enter `answer`. A music layer's gain enters by a **different door** and had no rule.

| row | source | before | after |
|---|---|---|---|
| a score's `MusicLayer { gain_db: NaN }` | S1 (`finite_or`), S2 (R6) | `layer_db NaN`, `Voice.base_db NaN`, sink at −80 dB, **`problems()` empty** | `layer_db 0.0`, `base_db 0.0`, `Applied 0.0`; one sentence |
| a `MusicLayers` move to `NaN` | S1 | `layer_db NaN`, `target NaN` | `0.0` |
| a layer started at `SILENT_DB` (guard) | S3 (`SILENT_DB` is finite) | −80 dB | **unchanged** — −80 dB |

**Against the sources.** S3 is why the neutral value is safe here: `SILENT_DB` is **finite**, so the guard leaves every
layer a score starts silent exactly where it is. Only a value that is not a number at all is normalised. That is the
same distinction iteration 18 drew between `chain_db`'s rule (for values the crate computed) and an input.

**What made this item rank 1 and shippable:** `MusicScore::problems` returns `Vec<String>`, while `CueTable::problems`
returns `Vec<CueProblem>`. A sentence is **not** an API change, so the report half needed no decision file — unlike
iteration 18's `CueProblem` carve. The fix is complete: behaviour and report, no carve left for the owner.

**Recovery, measured and recorded honestly.** A layer at `NaN` was *not* permanent: `approach(NaN, −3.0)` returns
`−3.0` — the crate's third answer to a non-finite number (a current value that cannot interpolate takes the target). So
the damage was a silent stem and an empty report for as long as a game did not happen to move the layer. That is why the
finding is rank 1 but the severity is stated as *silent until moved*, not *silent forever*.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack}.png`, the loudness table, both probe files.

**Audio unchanged, deliberately.** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS,
`footstep-grass` −18.4 LUFS / −6.1 dBFS (looped), `mix.wav` −4.3 LUFS / +0.2 dBFS — identical to iterations 14–18. The
attack spectrogram is unchanged: clean onset, cut at 0.033 s with no vertical line.

**The measured table**, `evidence/music-gain{,-before}.txt`:

```
before                                                   after
1. score layer NaN   layer_db NaN, base_db NaN, Applied -80.0, problems []   layer_db 0.0, base_db 0.0, Applied 0.0, one sentence
2. move to NaN       layer_db NaN, target NaN                               layer_db 0.0, target 0.0
3. layer at SILENT_DB  -80.0 (guard)                                          -80.0 (unchanged)
```

Row 1 shows the meter disagreeing with the sink in the before column — `base_db NaN` while `Applied −80.0` — which is
exactly the disagreement R6's second half forbids, and the fix removes it.

**A test defect caught here:** my move test used `MusicLayers::new`, which defaults to `When::NextBar` — 2 s at 120 bpm —
and asserted after 0.5 s, so the move never fired and the failure looked like a code bug. The test now says `.now()`
explicitly and its comment names the quantisation it is deliberately not testing.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** The score test builds a **one-layer** score at `tempo_bpm: 0.0` (no beat grid), a shape no other
music test uses; the move test exercises `When::Now` against the fixture's default `NextBar`; and the guard test asserts
`SILENT_DB` **survives**, which is the one thing a normalisation could plausibly break.

**Can a developer understand it without the code?** `docs/guide/music.md` gains the rule where a layer's gain is
described: a gain that is not a number is `0 dB`, `SILENT_DB` is untouched because it is finite, and `problems()` names
it. A panel drawing `layer_db` now draws a number.

**What a fresh reader would still ask:** *what about a `MusicLayers` move to a non-finite target — why no sentence?*
Because a message has no report to write into, and a second reporting mechanism for one message is worse than one; it is
covered by the same decision file iteration 18 wrote.

**Verdict: GREAT.**
