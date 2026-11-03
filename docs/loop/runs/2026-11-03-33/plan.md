# plan.md — iteration 33: a queued change follows the score's tempo

## The pick, and why it beats the runner-up

**Pick:** a `NextBar` change queued at 160 bpm fires at **1.5 s** after the score is edited to 80 bpm, where a bar is
**3.0 s** — the crossfade begins in the **middle** of a bar. `When::NextBar` is documented as *"on the next bar"* and the
guide's title is *"layers that change on the bar"*; a tempo edit is the ordinary path for a `Resource` *"the Cauldron
edits"*.

Beats the runner-up (**a Music block in the panel**, which iteration 32 measured at ~134 px against 12 free height): that one
is rank 4 and needs a layout decision. This one is rank 1 — a **music transition that lands off the beat** — it is silent, and
the fix is inside one struct.

## The design

`Pending` gains `when: When` and `tempo: f32`; `move_layers` re-quantises each change whose tempo is stale, before the
"due" filter:

- `at.max(now)` so a faster tempo cannot pull the change into the past;
- the `tempo` guard so an unchanged score does no work and no queued change creeps forward;
- `When::Now` returns its own instant — it has already been applied.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `Pending` is private. No public type changed: `MusicState`'s accessors
and `next_change_s` keep their signatures.

## Tests

- `a_queued_change_follows_the_scores_tempo` — queued at 160 (1.5 s), tempo to 80, `next_change_s` becomes **3.0**.
  **Fails on the base commit** (stays 1.5).
- `a_queued_change_holds_at_an_unchanged_tempo` — guard: thirty frames at the same tempo leave the instant **exactly**
  where it was.
- `a_requantised_change_still_fires_and_now_is_not_delayed` — guards: a `When::Now` change applies on its own frame, and a
  change re-quantised by a retempo still lands its target.

## The carve

- **Not** the Music block. Iteration 32 measured the column at **12 px** free against ~134 needed; it remains the next item,
  with its budget measured rather than guessed.
- **Not** a recompute of `at_s` on every frame: a per-frame re-quantise moves a queued change forward for ever, which the
  guard test exists to catch.
