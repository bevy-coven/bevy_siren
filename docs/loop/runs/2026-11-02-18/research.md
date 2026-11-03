# research.md — iteration 18: a gain that is not a number, and the crate's two answers to one caller mistake

## The item

The crate states its policy for a **non-finite caller value** in one place, and does the opposite in another:

**`docs/guide/spatial.md`:** *"A distance that is not a number, or is negative, is treated as `0.0` — a caller handing in
a `NaN` transform gets a sound at the listener's feet, not a `NaN` volume that silences the whole bus."* And
`model::distance_gain` is exactly that: `let d = if distance_m.is_finite() { … } else { 0.0 };` — a `NaN` distance is the
**neutral value**, the listener's feet.

**`model::chain_db`:** `if !db.is_finite() || *db <= SILENT_DB { return SILENT_DB; }` — a `NaN` gain is **silence**.

Both are caller data. One is neutralised to the value that means "no distance"; the other is neutralised to the value
that means "no sound". Measured:

```
a cue authored `gain_db: NaN`     problems() says nothing;  Played says gain -80.00;  the sound is silent
Play::cue("shot").gain(NaN)       problems() says nothing;  Played says gain -80.00;  the sound is silent
Play::cue("shot").pitch(NaN)      Played says pitch NaN;    the sound plays at speed 1.0  (semitones_to_ratio guards it)
```

## Sources

**S1. `docs/guide/spatial.md`** (the crate's own stated policy) and `model::distance_gain`. A non-finite caller value is
the **neutral** one for what it measures, not the silencing one.

**S2. R6 (vendored: `docs/research/siren.md`)** — *"a play is never silent: `Played` or `Refused`, both with the
reason."* A gain that becomes silence while `Played` reports a number breaks this directly: the answer says played, the
sound does not happen, and nothing anywhere says why.

**S3. `model::semitones_to_ratio`** — `if !semitones.is_finite() { return 1.0; }`. The **pitch** path already does the
right thing: a non-finite detune plays at normal speed. `Played.pitch`, however, reports the raw `NaN`, so the meter and
the sound disagree — the one thing R6's second half (*"everything a play decides is readable"*) forbids.

**S4. `db_to_gain` / `gain_to_db`'s doc** — a zero gain is `SILENT_DB`, *"never negative infinity"*, so a caller *"must
not be able to hand the next"* stage a value it cannot compare. The crate already refuses to let an infinity escape the
model; this iteration stops letting one **in** as silence.

**S5. Gregory §14.5.8 / the mix's one-writer rule**: a bus's level is one number, and a `NaN` bus level would silence
every voice on it. `chain_db`'s silencing rule is **right for a bus** (a level the crate computed) and **wrong for a
cue's own gain** (a number an author typed), because the two have different recovery: a broken bus is a mix fault the
crate must survive, a broken cue is an authoring fault the author must see.

## The technique

**Neutralise a non-finite gain where it enters, exactly as `distance_gain` neutralises a non-finite distance.** A gain
that is not a number means "no gain applied" — `0.0` dB, the neutral value — and the play is then **audible at the level
the cue actually authored**, rather than silent at a level nobody wrote. The pitch is normalised the same way
(`Played.pitch` reports the `0.0` that `semitones_to_ratio` already plays).

- `chain_db` keeps its rule for the values it computes (a bus at −80 dB silences the chain; that is the model's job).
- The **inputs** — `cue.gain_db`, `slot.gain_db`, `request.gain_db`, `request.pitch` — are normalised before they reach
  it, with one shared helper so the crate has **one** definition of "not a number means the neutral value".

**Why neutral rather than refusing?** A refusal is the right answer for a **request** the crate cannot honour, and a
`Refusal` variant is public API; a *normalisation* is what the crate already does for the one other non-finite caller
value it accepts (a distance), and doing it the same way makes the policy one rule instead of two. A refusal variant is
recorded in `needs-decision/` for the owner, because it is the stricter answer and a reasonable person could want it.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a non-finite distance is the neutral `0.0` m | `0.0` | S1, `distance_gain` |
| a play is never silent; `Played` or `Refused`, with the reason | — | S2 (R6) |
| a non-finite detune plays at `1.0×` | `1.0` | S3, `semitones_to_ratio` |
| a zero gain is `SILENT_DB`, never −∞ | `−80` | S4 |
| a bus level is one number; a `NaN` there silences the bus | — | S5 |

## What good sounds like

A cue authored `gain_db: NaN` **plays at the level it otherwise authored** — audible, not silent — and a game that wants
to know is told by `problems()` (**the seventh `CueProblem`**, `needs-decision/bad-gain-problem.md`). A `Play` override
that is not a number does the same: no gain, normal speed, and `Played.gain_db` / `Played.pitch` report the numbers the
sound is **actually at**, so the meter never disagrees with the device.
