# plan.md — iteration 60: a draw that cannot change its answer

## The pick, and why it beats the runner-up

**Pick:** a play draws for its detune **even when the cue's `pitch` range is exact**, so adding a fixed-pitch cue to a
scene moves every later pick for every other cue. Measured, pinned by a test.

Beats the runner-up (a `Refusal` for a wrong-bus stinger) because **that one is blocked** on a public-enum break, and this
is a measurement plus the documentation the crate needed either way.

## The ranked list

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
2. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
3. A stinger on the wrong bus writes only a log line, not a `Refused` — confirmed by probe in iteration 59; same
   decision.
4. **An exact-pitch play spends a shared-stream draw.** Measured, documented, and given a decision file this iteration.
5. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.

## Why nothing in `src/` changed

Six hypotheses were probed and **all found already correct**; each was deleted rather than kept, because a test asserting
untouched behaviour is noise:

- `SirenLog` stays chronological through the out-of-order eviction `push` performs;
- `semitones_to_ratio`'s ±4-octave clamp matches the mixer's `set_speed` clamp, so `Played.pitch` is the rate the device
  plays on both backends;
- `pitch` composes as `cue.pitch.at(rng) + request.pitch`, and `played_semitones` reports the clamped sum;
- `load_table` treats an asked key as not-missing and asks nothing the second time;
- a muted bus still plays, `Played` carries the cue's own gain, the bus reads silent;
- two music layers on a one-voice bus settle: one steals, the other is refused once and recorded as `blocked` — the
  documented virtual-voice state, not a fault. (My first probe asserted the wrong layer and failed; the code was right.)

## The design

- **A test that pins the coupling** — `an_exact_pitch_range_still_draws_from_the_stream` asserts the two sequences
  **differ**, with the decision file named in the assertion message, so a future fix flips it deliberately.
- **A guide sentence** in `## The seed`: the stream is shared, a play draws for its detune even when the range is exact,
  so play order is part of the seed's meaning.
- **`needs-decision/exact-pitch-draw.md`** — options A (keep and document, shipped), B (skip the draw, recommended for the
  next minor), C (two streams), with the blast radius of each.

## Fields

**No new field, no API change, no behaviour change.** Documentation and a test.
