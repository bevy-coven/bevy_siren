# plan.md — iteration 17: the backend flag does not decide ownership on its own

## The pick, and why it beats the runner-up

**Pick:** `spawn_voice` takes "the mixer owns playback" from `Backend(true)` alone. With no `Output`, the voice gets no
`AudioPlayer` and has no mixer, so it cannot sound, cannot end, and cannot be retired — it holds a budget slot forever
while `Played` says it played. Rank 1: silent wrong output **and** a leak, reachable through a public resource.

Beats the runner-up (documenting a `Refusal::NotALoop`): that one is a message; this one is a sound that never happens
and a budget that never comes back.

**The two-backend hunt at the end of iteration 16 came back clean** on the two things worth checking — a **steal** logs
identically on both paths, and the **mix** (`bus_db`, `ducked`) is identical field for field under a scripted duck. The
defect was one level down: not "do the backends answer the same?" but "can a backend answer nothing at all?".

## The design

One line, in `play_requests` (`src/play.rs`), where the voice is spawned:

```rust
let mixer_owns = backend.0 && output.is_some();
```

- **No new field.** No `CueTable`, `SirenSettings` or `MusicScore` change, no magic constant: the predicate is the
  documented rule itself.
- **No public API change.** `Backend` keeps its tuple field, so `Backend(true)` in the crate's own examples and in
  any host still compiles.
- **The doc already promised it.** `start_pending`'s doc says Bevy's `AudioPlayer` owns every voice without an `Output`;
  the spawn now reads the resource that sentence is about.

## Tests

- `a_backend_with_no_output_still_gives_the_voice_a_player` — `Backend(true)`, `Output` removed, one play: the voice
  carries an `AudioPlayer`, and the play is answered. **Fails on the base commit.**
- `a_backend_with_an_output_leaves_the_voice_to_the_mixer` (behind `mixer`) — the other half: with an `Output`, the voice
  has **no** `AudioPlayer`, so the two paths cannot both play it. Passes before and after; it pins the behaviour the fix
  must not change.

## The carve

- **Not** a `Backend` API change (a method instead of a field): public API, and unnecessary.
- **Not** a refusal for the desync: the fallback exists, is documented, and now works — a lost sound would be the worse
  outcome.
