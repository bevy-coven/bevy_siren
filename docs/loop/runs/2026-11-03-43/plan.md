# plan.md — iteration 43: a parented sound says it stopped when its owner is despawned

## The pick, and why it beats the runner-up

**Pick:** `Play::on(entity)` parents a voice, so despawning the owner is the ordinary end of a sound on a vehicle or a
torch — and **neither backend logged it.** The mixer freed the slot silently; Bevy's path had no entity left to notice.
A log whose last word about a sound is `played`, while the mix counts one fewer voice, is the silent-failure class this
crate exists for.

Beats the runner-up (a live-layer count in the `Music` header): that is decoration; this is a **missing line for an end
that happens in every game**, and the crate's own vocabulary (`"ended"`, `"stop"`, `"cue stopped"`, `"stolen by …"`)
already logs every other end.

## The blocked items, re-checked

- **`cues.yaml` case 31** (a non-finite cue `gain_db` reported by `problems()`) — still blocked on
  `needs-decision/bad-gain-problem.md`; every route touches a public enum with no `#[non_exhaustive]`.
- **`Completes`** — `needs-decision/remove-completes.md` is **not** blocking anything: `retire_voices` already reads
  `AudioOutput::finished`, so only the dead type waits. The backlog had it labelled as blocking this item; it does not.

## The design

ONE new system, `mix::log_despawned_voices`, registered `.after(SirenSystems::Mix)`:

1. Each frame it records every voice it sees — `Entity → (cue name, had a ChildOf)` — in a `Local<BTreeMap<…>>`.
2. It reads `RemovedComponents<Voice>`; an entity that vanished **and was parented** earns one
   `Entry::Stopped { why: "owner despawned" }`.
3. `start_pending` **stops** writing the line (it did in the first draft, and the mixer logged twice — caught by test).

## Fields

**No new field** on `CueTable`, `SirenSettings` or `MusicScore`. **No new enum variant**: the reason is a sentence, per
`Entry::Stopped`'s design. No public type added, changed or removed.

## Modules touched

- `src/mix.rs` — `log_despawned_voices`, and `EndedVoice` left as it was.
- `src/lib.rs` — one registration.
- `src/playback.rs` — the comment saying why the sentence is *not* written there.
- `tests/play.rs` — `a_despawned_owner_stops_its_sound_and_says_so` (Bevy's path, no features).
- `tests/mixer.rs` — `a_despawned_voices_mixer_slot_is_freed` now also asserts one `owner despawned` line.
- `examples/mixer.rs` — a `parent` step.

## Tests

- `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` — **no feature flags**, so it runs on the default
  backend: one line naming the sound, and the voice gone.
- `tests/mixer.rs::a_despawned_voices_mixer_slot_is_freed` — the mixer path, asserting the slot is freed **and** exactly
  one line exists (the double-log guard).
- `tests/play.rs::a_stop_for_an_ended_voice_is_a_no_op` — the guard that an **unparented** despawn logs nothing; the
  first draft reddened it, and `ChildOf` gates the line.
- **Mutation:** replacing the `log.push` with nothing reddens both the Bevy-path and the mixer test (verified).

## The carve

- **Not** a new `Entry` variant — a public enum, so a decision file, not an iteration.
- **Not** a `ChildOf` check inside `retire_voices`: measured, it cannot work. Bevy defers the despawn, so during `Mix`
  both the owner and the voice still exist, and one frame later both are gone. The removal reader is the only place the
  answer is visible.
- **Not** a hook on `Voice` (`on_despawn`): it would change a public component's behaviour for every consumer.
