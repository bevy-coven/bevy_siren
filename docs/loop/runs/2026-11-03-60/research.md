# research.md — iteration 60: a draw that cannot change its answer

## The item

`answer` computes the cue's detune **before** it can know whether the cue's range can use the number:

```rust
let mut pitch = cue.pitch.at(choices.rng.next_f32());   // draws, always
```

`PitchRange::exact(n)` is `{ low: n, high: n }`, and `PitchRange::at(unit)` is `low + (high - low) * unit` — with
`low == high` the unit **cannot** affect the result. So the draw is provably useless, and it is spent from
`SirenChoices::rng`, the one stream every cue shares.

**Measured** (`tests/play.rs::an_exact_pitch_range_still_draws_from_the_stream`): an app that plays an exact-pitch cue
four times before a `Shuffle` cue produces a **different** six-play variant sequence than an app that never played the
first — and the test asserts they differ, so the coupling is pinned rather than assumed.

## Why this is a finding and not a patch

`SirenSettings::seed` is the project's reproducibility lever: `docs/guide/cues.md` promises *"the same number plays the
same sequence twice, field for field, in two processes"*. A draw a cue cannot use means **adding a fixed-pitch cue to a
scene shifts every later pick for every other cue** — a project's soundscape is a function of its seed **and its play
order**, not its seed alone.

Skipping the draw is a three-word change. It is not an iteration's call, because it **re-rolls every project's
soundscape in the build that ships it**, with no authoring change to explain it. That is a release note.

## Sources

**S1. `PitchRange::at`'s own arithmetic** (`src/cue.rs:167`) — `low + (high - low) * unit`. With `low == high` the
expression is `low` for every `unit` in `[0, 1]`. The draw provably cannot move the answer.

**S2. `SirenSettings::seed`'s doc and the guide's promise** — the stream is *"the project's file, not a crate
constant"*, and the determinism test asserts a seed replays field for field. A draw the answer does not need is a leak of
this guarantee into *play order*.

**S3. Iteration 51's precedent** — a picker whose window was the allocator's rather than the author's. Same shape: a
number used where a decision was meant. That one was fixed because no stream was observable; this one changes the
observable stream.

**S4. `Cue::pitch`'s default** — most cues in a shipped table are random, so the draw is *usually* needed. The waste is
per-cue, not per-play, and the effect is only on the cues whose range is exact — which is exactly the cue an author
writes when they want no variation, and therefore the one they least expect to change anything.

## The technique

Recorded as **`needs-decision/exact-pitch-draw.md`** with options A (keep and document — shipped), B (skip the draw when
`low == high` — recommended for the next minor), and C (separate pitch and variant streams). The guide gains the sentence
the crate needed regardless of which option is chosen.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| draws per play | 1 for the variant, 1 for the detune | `answer` |
| a detune draw with `low == high` | cannot change the answer | S1 |
| measured effect on a later cue's six-play sequence | **different**, same seed | measured |
| the same seed, two processes | identical sequence, field for field | S2 |

## What good sounds like

Nothing audible changes today. What a reader gains is the fact that **play order is part of the seed's meaning**, so a
scene that adds a fixed-pitch cue is not an A/B of the audio — and the decision file is where the fix waits with its
release note.

## Blocked

`hs distill search` answered `500 … gpu busy` for the tenth iteration running. The evidence here is the crate's own
arithmetic and the guide's promise; no DOI bears on a draw that cannot affect its result.
