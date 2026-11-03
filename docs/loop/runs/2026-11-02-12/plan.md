# plan.md — iteration 12: a stale voice handle must not take the game down

## The pick, and why it beats the runner-up

**Pick:** `StopVoice` for an entity that has already been despawned **panics** the game. `stop_requests` takes the entity
straight from a message and calls `commands.entity(entity).insert(..)`; Bevy 0.20's default error handler panics on a
despawned entity. The crate's own `retire_voices` despawns the entity the frame a one-shot ends, and `Played::entity` is
documented as the handle to keep for `StopVoice` — so the crate hands out a handle and then crashes when it is used.

**Runner-up:** the five decision files, all public-enum questions for the owner.

The pick wins on the loop's first criterion at its most severe: not wrong audio, a **crash**, reachable from the crate's
own documented workflow and from its own normal retirement path.

**Carved off:** nothing. One guard.

## Design

`src/mix.rs`, `stop_requests`, one loop body:

```rust
for (entity, why) in requests {
    // Resolve against the live query first. A `StopVoice` may name a voice the crate has already retired — the game
    // holding `Played::entity` cannot know a one-shot ended — and a command on a despawned entity panics Bevy's default
    // error handler. Stopping something that has stopped is not an error to report; it is a race, and the answer is
    // that there is nothing left to stop.
    let Ok((_, voice)) = voices.get(entity) else { continue };
    commands.entity(entity).insert(Stopping { since_s: now, why: why.to_owned() });
    log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: why.to_owned() });
}
```

`voices` is already `Query<(Entity, &Voice), Without<Stopping>>`, so this also skips a **second** stop for a voice already
fading — a fade is begun once, which the crate already intended and nothing asserted.

`StopCue` is untouched: it iterates the same query, so its entities are live by construction.

## Every field touched on a serde type

None.

## Tests that will prove it

`tests/play.rs`:

1. `a_stop_for_an_ended_voice_is_a_no_op` — play a one-shot, despawn the voice the way `retire_voices` does, then send
   `StopVoice` for that entity. **Today this panics inside `app.update()`**; after, the app updates, nothing is logged,
   and the world is unchanged. This is the regression test and the reason for the change.
2. `a_second_stop_for_a_fading_voice_is_skipped` — `StopVoice` twice for a live loop: one `Stopped` line, not two; the
   voice still fades out.
3. `stop_cue_still_stops_every_voice_of_a_cue` — the name path, unchanged, so the fix cannot narrow it.

`tests/mixer.rs` keeps its existing stop coverage; no change.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (cases 7–9 and 20–21 touch the budget and retirement);
- `cargo run --release --example perfcheck` before/after — one `Query::get` per stop request, and stop requests are rare;
- `examples/render` before/after: cue WAVs byte-identical;
- the probe that found it — play, despawn, stop — run against the fixed tree.

## Risk

A stop for a **despawned** entity now logs nothing, where a game might have wanted a line saying "you stopped something
that had ended". That would be a refusal-shaped message about an event that is not an error, and the loop's own rule is
that the log holds decisions about sounds — so silence is right, and the doc on `StopVoice` says so. If the owner wants a
line, it is the same `Entry` decision that `needs-decision/non-spatial-position.md` waits on.
