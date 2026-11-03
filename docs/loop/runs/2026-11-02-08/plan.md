# plan.md — iteration 8: a position on a non-spatial cue is not a panner

## The pick, and why it beats the runner-up

**Pick:** `spawn_voice` decides Bevy's `PlaybackSettings.spatial` — and whether to attach a `Transform` — from
`request.position.is_some()` alone, ignoring `cue.spatial`. A cue authored `spatial: false` and played with
`.at(position)` therefore goes through the two-ear panner, which applies `min(1, 1/d²)` as well as a pan: **−38.7 dB at
8 m**, while `Played.gain_db` and the log both report **0.00 dB**. The meter is wrong, so nothing downstream can
notice.

**Runner-up:** the three decision-blocked items (an undecodable variant, a cap in the refusal payload, removing
`Completes`). All need the owner's answer; none is buildable today.

The pick wins on the loop's first criterion — wrong output — and on its worst sub-class: a `Played` whose own number
disagrees with the sample. It also closes an internal contradiction: one predicate, three places, two of them wrong.

**Carved off:** refusing a non-spatial cue given a position. That needs a `Refusal` variant (a public enum change), so
`needs-decision/non-spatial-position.md` carries it; this iteration makes the behaviour honest without a break.

## Design

`src/play.rs` only, plus a log entry the crate already has.

- `Started` gains `spatial: bool` — the resolved "this voice is panned and placed" fact, computed once in `answer`
  under the predicate it already uses (`cue.spatial && request.position.is_some()`).
- `spawn_voice` uses `started.spatial` for `PlaybackSettings.spatial` and attaches `Transform` only when it is true.
- When a play has a position and the cue is not spatial, `play_requests` writes one `Entry::Refused`-adjacent line —
  no: a refusal would be a lie, since the play **did** happen. It writes an `Entry::Played` as usual and the log gains
  a plain line via a new `Entry` variant? That is a public enum change too.

  **Simpler and still honest:** the `Played` message already carries the true gain; the log's `Played` line carries the
  same. Nothing needs adding to say the position was ignored, because the *observable* is that `Played.gain_db` is true
  and the sample matches it. A project that passed a position by mistake can see `spatial: false` in its own file.
  Recorded as carved off; if a log line is wanted it is the same decision file.

## Every field touched on a serde type

None.

## Tests that prove it

`tests/play.rs`:

1. `a_non_spatial_cue_ignores_a_position` — `spatial: false`, `.at((0,0,8))`, a listener present: `Played.gain_db` is the
   cue's own gain (0 dB), the spawned entity has **no** `AudioPlayer` flagged spatial and **no** `Transform`, and — the
   observable that matters — the voice is not routed through the panner. Fails on the unfixed tree, where the entity's
   `PlaybackSettings.spatial` is `true` and a `Transform` is attached.
2. `a_spatial_cue_at_a_position_is_still_panned_and_compensated` — the case that must not regress: `spatial: true`,
   `.at(8 m)`, the gain reads −6.02 dB, `Compensation` is non-zero, and the entity carries a `Transform` at the
   position.
3. `a_spatial_cue_without_a_position_is_still_flat` — the documented case: `spatial: true`, no `.at`, gain is the cue's
   own, no panner.
4. `a_spatial_cue_with_no_listener_is_still_refused` — `SpatiallyMute`, unchanged.

`tests/model.rs`:

5. `the_panner_law_is_what_a_spatial_cue_divides_out` — `backend_spatial_gain` at 8 m on-axis is ~0.0117 (−38.7 dB), the
   number this iteration's probe printed, so the magnitude of the bug is pinned where the arithmetic lives.

`examples/render` gains a line naming the panner's gain at 8 m beside the cue's attenuation at 8 m — the two numbers
that must differ by exactly the compensation for a spatial cue, and that a non-spatial cue must not see at all.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 14 is the distance law and must stay green);
- `cargo run --release --example perfcheck` before/after;
- `examples/render` before/after: cue WAVs byte-identical (no cue that plays correctly moves);
- a probe that measures the **audible** gain the panner would apply, not only `Played.gain_db`, so the 38.7 dB gap is a
  number in the evidence rather than an inference.

## Risk

A project that relied on the accident — `spatial: false` cues played with `.at()` to get a pan — loses that pan. There
is no such documented behaviour, the meter was wrong while it happened, and the fix is the one the field's doc
describes. It is called out in the `CHANGELOG` in those words.
