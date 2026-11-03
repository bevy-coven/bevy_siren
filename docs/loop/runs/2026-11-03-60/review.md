# review.md — iteration 60, three passes

## Pass 1 — Function: does a draw that cannot change its answer exist?

**The claim** (`src/cue.rs:167`, `PitchRange::at`): `low + (high - low) * unit`. With `low == high` — what
`PitchRange::exact(n)` builds — the expression is `n` for **every** `unit` in `[0, 1]`. The draw cannot move the answer.

**Measured** (`an_exact_pitch_range_still_draws_from_the_stream`): an app that plays an exact-pitch cue four times before
a `Shuffle` cue produces a **different** six-play sequence than one that never played it. Same seed, same table, same
order for the second cue.

So `SirenSettings::seed` alone does not determine the soundscape: **play order does**. `docs/guide/cues.md` promised the
seed was the lever, and now says what else is.

**Verdict: GREAT.**

## Pass 2 — Zoom: why this is documented rather than patched

The fix is three words — `if low == high { low } else { at(rng.next_f32()) }`. Shipping it would **re-roll every
project's soundscape** in the build that carried it, with no authoring change to explain the difference. A capture
compared against the previous build would show different variants for the same seed, and the release note would be the
only warning.

That is the bar this loop uses for a `needs-decision` file: not doubt about the fact, but **blast radius on a published
crate** (0.1.0, nine downloads). `needs-decision/exact-pitch-draw.md` carries A (keep and document — shipped), B (skip
the draw — recommended for the next minor), C (two streams), each with its cost.

**The test is written to flip.** Its assertion is `assert_ne!(alone, after_exact)`, with the decision file named in the
message — so the day option B ships, the test fails and points at the file that explains why.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: six probes, and one that taught me the code was right

Six hypotheses were probed and all found correct: log chronology through out-of-order eviction; the pitch clamp agreeing
with the mixer's `set_speed` on both backends; pitch composition with a `Play` override; `load_table`'s ask semantics; a
muted bus; and two music layers on a one-voice bus.

**The sixth failed first, and the failure was mine.** I asserted `blocked("combat")` — but `calm` starts first, `combat`
steals it, and `calm` is the one refused and recorded. The log said so plainly:

```
played `track.calm` → stopped `track.calm`: stolen by track.combat (bus) → played `track.combat`
→ refused `track.calm`: no voice free on bus `Music`
```

That is the documented virtual-voice path (`Gregory §14.5.8`) doing exactly what it says. I deleted the probe rather than
inverting it, because the crate already has `tests/music.rs::a_refused_layer_start_is_tried_again_when_room_frees` —
case 33 — and a second test of the same behaviour is noise.

**Nothing audible moved**, which is the claim for a documentation iteration: `bus-Music` unchanged, `mix.wav` at
−18.6 LUFS / −6.1 dBFS, and the cue spectrogram a clean transient with no clipping shelf.

**Verdict: GREAT.**
