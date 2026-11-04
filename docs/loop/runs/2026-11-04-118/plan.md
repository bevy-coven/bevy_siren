# Iteration 118 — plan: a play is answered once, and the entity `Played` names is not destroyed under the game

## Ranked

| # | item | rank | why it is (not) this pass's |
|---|---|---|---|
| 1 | **On the crate's own mixer, bytes `rodio` refuses are `Played` and `Refused` in one frame, the entity despawned in it — a game that hangs data on `Played::entity` panics** | bug (crash on a documented use; the crate's own rule *every play answers exactly one of `Played`/`Refused`*; three sentences — README, crate docs, `AGENTS.md` — false) | **this one**: reproduced and deterministic, the fix is general (ask the backend that will decode), small, and additive |
| 2 | The README / crate-root quick start does not play a sound as written (keys with no extension; `walk` and `hear` never scheduled; `tests/readme.rs` holds text equality only) | docs / front door | a loud failure since 116 (every play is refused with its cause in the log), so below a crash; **first in the next pass**. Evidence: `evidence/before/front-door-probe-on-the-unchanged-tree.txt` |
| 3 | `Play::on(owner)` for an owner already gone: a voice with no `ChildOf`, an ownerless loop for ever, a leaked slot | bug (silent) | needs an owner despawned before its play. An existence check at play time is **unsafe** (an entity spawned in the same frame through deferred commands looks gone — valid plays would be refused). Safe design: a crate-private `Owner(Entity)` component and a system that stops a voice carrying `Owner` and no `ChildOf`. Carved off |
| 4 | The layers of a score start apart (3 of 100 warm-cache starts; design in `runs/2026-11-04-116/plan.md`) | behaviour | carry-over |
| 5 | A person listens | owed | carry-over |

**Why 1 beats 2**: a crash in a game's own system, naming no file, on a use the field's doc names, against a documentation defect that already fails loudly. **Why 1 beats 3**: a deterministic crash against a leak that needs a rarer order of events, and the same fix does not cover it.

## Design

`AudioOutput` (public trait, already documented as *a game never implements it*) gains a **defaulted** method — the question *can your `play` decode these bytes?*:

```rust
fn decode_error(&self, _bytes: &Arc<[u8]>) -> Option<String> { None }
```

- `MixerBackend` overrides it: `rodio::Decoder::new(Cursor::new(Arc::clone(bytes))).err().map(|e| e.to_string())` — the construction `play` makes, **without `play`'s `to_vec()` copy**.
- `sounds::Decode<'a> { Skip, Bevy, Output(&'a Output) }` says who is asked. `cannot_play(files, sounds, handle, &Decode)` is the one question (it was `bevy_decodes: bool`); `play_requests` builds the `Decode` once per frame — `Output` when `Backend(true)` and an `Output` exists, `Bevy` otherwise — and passes it to `answer` and `refusal_sentence`; `refuse_unplayable_voices` asks `Bevy` for a voice with an `AudioPlayer` and `Skip` for one without.
- `SirenSounds` holds **one memo per asker** (`Mutex<(Verdicts, Verdicts)>`): the verdict is the asker's own, never the other backend's. The two decoders agreed on 14 of 14 files, but an agreement is a measurement and not a contract.
- A play whose bytes are loaded and refused is `Refused { MissingSound }` **at the play**, the sentence the 117 machinery already writes (the decoder's words and the hint). **No new `Refusal` variant**, no new message, no new field.
- **In flight stays as it was**: a voice spawned while its bytes load is refused by `start_pending` (mixer) or `refuse_unplayable_voices` (Bevy) in the frame they arrive; `Played` could not have known.
- `Played::entity`'s doc states what a game may rely on (it exists when the message is written; a voice stolen or stopped in the frame it was announced is despawned in that frame, with a `stopped` line) — found by the invariant test's first form.

`needs-decision/audio-output-decode-error.md` records the additive trait method (options, edge cases, recommendation).

## Tests (written first; red on the unchanged tree)

- `tests/one_answer.rs::bytes_the_crates_mixer_refuses_are_refused_at_the_play_and_nothing_is_announced` — a fake output that refuses, a tagging game system, three frames: no panic, 0 `Played`, one `Refused`, 0 voices, 0 counted, `play` never called, the log says *cannot be decoded* and never *played*.
- `tests/one_answer.rs::every_play_is_answered_once_and_a_voice_that_is_gone_is_explained` — six scenarios (Bevy's path; the mixer; a steal and a stop in one frame; two plays of one cue in one frame; a play stopped in the frame it was announced; the refused one): after every frame, one answer per play, and a `Played` entity that is gone is explained by a `Stopped` entry.
- `tests/one_answer.rs::a_sound_is_asked_of_the_mixer_once_however_often_it_is_played` — five plays, one question; an edit is a new buffer, asked again once.
- `tests/one_answer.rs::a_verdict_is_the_askers_own` — the mixer accepts four zero bytes, Bevy refuses them: with `Backend(false)` the same bytes are refused, not served from the mixer's memo.
- `tests/mixer.rs::the_real_mixers_decode_error_agrees_with_its_play` — ten kinds of file on the real backend (needs a device): `decode_error` is `None` exactly when `play` returns `Some`, five decode and five do not.
- **No existing test is edited.** The fakes in `tests/mixer.rs`, `tests/failed_load.rs` and `examples/budget.rs` inherit the default.

## Carved off

Items 2 and 3 above, with their evidence. Console `warn!` output for a refused sound was **decided against**: `src/log.rs:7` says *"Nothing here is a `println!`. A crate that prints is a crate a game cannot silence"*, and the crate prints one line (`mixer/mod.rs:95`, no audio device). Recorded as a candidate in the log.

## Predictions

In `research.md`, written before the fix was built (the probe that found the defect preceded them, and says so).
