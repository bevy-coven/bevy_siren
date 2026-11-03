# research.md — iteration 19: a music layer's own gain is the one number with no rule

## The item

Iteration 18 gave the crate **one rule** for a non-finite caller value — it is the neutral value for what it measures
(`model::finite_or`) — and applied it to the four numbers that enter `answer`. A music layer's gain enters by a
**different door** and has no rule at all:

```
MusicLayer { name: "calm", cue: "t", gain_db: NaN }
  MusicState::layer_db("calm")  = Some(NaN)
  MusicState::target_db("calm") = Some(NaN)
  Voice.base_db                 = NaN
  Applied (written to the sink) = -80.0 dB     <- chain_db silences it, as it must
  MusicScore::problems(&table)  = []           <- and nothing says so
```

The layer is **silent until a game moves it**: `approach(NaN, -3.0)` returns `-3.0` (its own guard), so a later
`MusicLayers` recovers it — measured, 200 frames later the layer is at `-3.0 dB` and `base_db` agrees. So the damage is
bounded, but it is a **silent stem and a silent report** for as long as the game does not move a layer it has no reason
to think is broken.

## Sources

**S1. `model::finite_or` and `docs/guide/buses.md` *(iteration 18)*** — *"one rule for a caller value that is not a
number: it is the **neutral** value for what it measures, never the silencing one."* A layer's starting gain is a caller
value in exactly this sense; the rule was applied to five numbers and not to this one.

**S2. R6 (vendored: `docs/research/siren.md`)** — *"a play is never silent: `Played` or `Refused`, both with the
reason"*, and *"everything a play decides is readable"*. A layer at `NaN` is a silent stem whose `layer_db` reports
`NaN`, so a panel drawing the fader draws `NaN`.

**S3. `model::approach` and `model::tau_for_fade`** — both already guard a non-finite input (`approach` returns `target`;
`tau_for_fade` returns `MIX_TAU_S`). The crate therefore has **three** answers to a non-finite number: neutral
(`distance_gain`), jump-to-target (`approach`), silence (`chain_db`). The difference is what the number *is*: a current
value that cannot be interpolated takes the target; an input that cannot be scaled takes the neutral value. A layer's
**starting gain** is an input, so it takes the neutral value.

**S4. `MusicScore::problems`** — it already reports a bad tempo (*"tempo_bpm is negative or not a number"*) and a bad
bar, a layer with no name, a layer listed twice, a layer naming a cue the table lacks, and a layer naming a cue that is
not a loop. A gain that is not a number is the seventh score fault, and it is the only one that silently silences the
stem instead of refusing it.

**S5. Khan et al. 2023, *Adaptive Background Music for a Fighting Game*, arXiv:2303.15734** (vendored: `siren.md` S3) —
adaptive music is **per-instrument gain keyed to game state**. The gain is *the* control surface of the whole
technique; a gain that is not a number is the technique's control having no value, which is precisely the case the
crate's report exists to name.

## The technique

1. **Normalise where the layer is created and where it is moved**, with the same `finite_or` iteration 18 added:
   a layer's starting `gain_db` and every `MusicLayers` target are read as `0.0` dB when they are not numbers. The layer
   then **plays at the level it would have had**, and `layer_db` / `target_db` / `Voice.base_db` agree with the sink.
2. **Report the score fault in `problems()`** — but `CueProblem` is a public exhaustive enum, and a
   **`MusicScore::problems` fault is a `String`**, not a `CueProblem` (the score's report is `Vec<String>`, unlike the
   table's). So the seventh score fault is a **sentence**, which is **not** an API change. This is the difference from
   iteration 18's `CueProblem` carve: the music report already speaks in strings, and adding a sentence to a `Vec<String>`
   breaks nothing.

**Both halves, no API change.** That is unusual for this crate and it is why this item ranks first: the fix is complete
and shippable without a decision file.

**What a `MusicLayers` target that is not a number does.** It is a **request** the crate cannot honour; iteration 18's
decision file (`needs-decision/bad-gain-problem.md`) already records the refusal-vs-neutralise question for a play. The
same answer applies and the same file covers it — normalise, and say so in `problems()` for the *score*; a
`MusicLayers` message has no report to write into, so its sentence is the log, as iteration 16 did for a one-shot layer.
This iteration writes the normalisation; adding the log line for a bad move is noted but is the same class as the
decision file's option C and is not a second mechanism.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a non-finite caller value is the neutral one | `0.0` dB | S1 |
| a play (or stem) is never silent; what it decides is readable | — | S2 |
| a current value that cannot interpolate takes the target | `target` | S3, `approach` |
| the score's report is `Vec<String>` | — | S4 |
| a stem's gain is the whole control surface of adaptive music | — | S5 |

## What good sounds like

A score whose layer says `gain_db: NaN` starts that layer at **0 dB** — the level it authored, had the number been a
number — and `MusicScore::problems` carries one sentence naming the layer. A panel drawing `layer_db` draws a fader at
0 dB and a warning beside it, rather than a fader at `NaN`. A `MusicLayers` move to a non-finite target arrives at 0 dB
instead of freezing the stem.
