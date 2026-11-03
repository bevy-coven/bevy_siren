# plan.md — iteration 18: one rule for a caller value that is not a number

## The pick, and why it beats the runner-up

**Pick:** the crate has **two opposite answers** to the same caller mistake. A non-finite **distance** is the neutral
value (`0.0` m, the listener's feet) and is documented as such; a non-finite **gain** is silence, undocumented, and
`Played` reports it as an authored level. A cue with `gain_db: NaN` therefore plays **silent while saying it played**,
which R6 forbids outright, and `Played.pitch` reports a raw `NaN` for a sound that actually plays at `1.0×`.

Beats the runner-up (`Refusal::NotALoop`): that one is a log line for a mistake already enforced; this one is a
**silent play reported as played**, with the meter disagreeing with the device.

## The design

One helper, one rule, at every place a caller's number enters:

```rust
/// A caller's gain, with a value that is not a number read as **no gain applied** — the neutral `0.0` dB.
pub fn finite_or(db: f32, neutral: f32) -> f32 { if db.is_finite() { db } else { neutral } }
```

- `answer` normalises `cue.gain_db`, `slot.gain_db`, `request.gain_db.unwrap_or(0.0)` and the final `pitch` with it, so
  `Started.gain_db` and `Started.pitch` are the numbers the voice is **actually at** — `Played` cannot disagree with
  the sink.
- `chain_db` and `bus` levels are **untouched**: a bus level the crate computed at `SILENT_DB` still silences the chain,
  which is the model's job and the mix's protection.

**No public API change.** No new `CueProblem`, no new `Refusal`. The stricter answers (a seventh `CueProblem`, a
`Refusal` variant for a bad request) go to `needs-decision/bad-gain-problem.md`, because both enums are exhaustive and a
new variant breaks every consumer's `match`.

## Fields

None. No `CueTable`, `SirenSettings` or `MusicScore` change. The number `0.0` is not a tunable: it is the identity of
addition in dB, which is what "no gain" means (R6's own chain rule: a stage of `0.0` dB changes nothing).

## Tests

- `a_gain_that_is_not_a_number_is_no_gain` — a cue with `gain_db: NaN` plays, and `Played.gain_db` is the level the cue
  would have had without the `NaN` (`0.0`), **not** `SILENT_DB`. Fails on the base commit (`−80.00`).
- `a_plays_gain_override_that_is_not_a_number_is_no_gain` — the same for `Play::gain(NaN)`.
- `a_plays_pitch_that_is_not_a_number_reports_the_speed_it_plays_at` — `Played.pitch` is `0.0`, which is the `1.0×` that
  `semitones_to_ratio` plays. Fails on the base commit (`NaN`).
- `a_silent_bus_still_silences_its_voices` — the **guard**: `chain_db`'s rule is untouched, so a bus at `SILENT_DB` still
  silences; a fix must not make a muted bus audible. Passes before and after.

## The carve

- **Not** a `CueProblem::BadGain`: public exhaustive enum; `needs-decision/bad-gain-problem.md`.
- **Not** a `Refusal` variant for a bad request: same, in the same file.
- **Not** any change to `chain_db` or to bus levels: the model's silence rule is correct and is what keeps a `NaN` bus
  level from silencing a game.
