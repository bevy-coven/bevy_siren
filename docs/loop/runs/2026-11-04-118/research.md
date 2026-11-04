# Iteration 118 — research: `Played` is followed, in the same frame, by a despawn of the entity it announced

*The finding came first, from a probe (`evidence/before/front-door-probe-on-the-unchanged-tree.txt`) — as in iteration 116, and said here rather than hidden. The predictions below are for the fix and were written before it was built.*

## What reading and running found

- **`Played::entity` is documented as a handle a game keeps**: *"The voice entity, for [`StopVoice`] or for a game that hangs its own data on it"* (`src/play.rs:93`).
- **On the crate's own mixer, a voice is spawned and `Played` is written in `play_requests`, and the bytes are decoded in `start_pending`, the next system in the same chain** (`playback.rs:35-108`). When `rodio` refuses them, `start_pending` writes `Refused`, **despawns the entity in that same frame** and leaves the `Played` standing. Measured with a fake output whose `play` answers `None` and a game system after `SirenSystems::Play` that does `commands.entity(played.entity).insert(Tag)`: **a panic**, in `bevy_ecs-0.20.1/src/error/handler.rs:128` — *"Entity despawned: The entity with ID 45v0 is invalid"* — naming the game's own system and not the file. (Bevy 0.20's `Commands::insert` is fallible and its default handler panics; only `try_insert` is silent: `system/commands/mod.rs:1411`, `:1602`.)
- Two answers to one play (`Played`, then `Refused`) also break the crate's own rule: *every play answers exactly one of `Played`/`Refused`; nothing plays on a refusal* — and the sentence repeated in the README, the crate docs and `AGENTS.md`, *"both backends answer `Played`, `Refused` and `Voice` identically"*. On Bevy's path iteration 117 refuses such bytes **at the play**; the mixer's path still does it after.
- **Two more defects were found by the same probe and are carved off, with their evidence**: the README quick start does not work as written (keys carry no extension, `walk` and `hear` are never scheduled), and `Play::on` for an owner that is already gone leaves an ownerless loop on both backends.

## Sources

| # | source | what it gives |
|---|---|---|
| S1 | `bevy_ecs-0.20.1/src/error/handler.rs:128` and `src/system/commands/mod.rs:1411`, `:1602` | a command on a despawned entity is an error handled by the default handler, which panics; `try_insert` ignores it |
| S2 | `src/play.rs:93` (`Played::entity`'s doc), `src/playback.rs:35-108` (`start_pending`), `src/output.rs:35-37` (`AudioOutput::play`: *"`None` when the sample cannot be decoded"*) | the promise, and the place it is broken |
| S3 | Gregory, *Game Engine Architecture*, 3rd ed., **§16.5.3 Handles** (home-still `Game Engine Architecture, Third Edition…`) | a handle is a reference that can go stale — *"If an object is deleted, it can simply null out its entry"*; *"handles are prone to the possibility of referencing a stale object"*; Bevy's `Entity` carries the generation that Gregory's unique id plays (the panic's *"its index now has generation 1"*). The crate hands a handle out, so **not handing out one it has already destroyed** is its half of the contract |
| S4 | Hunt and Thomas, *The Pragmatic Programmer*, 20th anniversary ed., **Tip 37 "Design with Contracts"** (home-still `hunt_pragmatic_programmer_20th`) | a routine *"shall guarantee that all postconditions and invariants will be true when it completes … failure to live up to the contract is a bug"* — `play_requests`' postcondition is **a `Played` names an entity that exists** |
| S5 | Barr, Harman, McMinn, Shahbaz and Yoo, *The Oracle Problem in Software Testing: A Survey*, IEEE TSE 41(5), DOI 10.1109/tse.2014.2372785, §4.1 | *"invariants … any test case that causes a program to violate an invariant has discovered an incorrect behavior; therefore, these invariants are partial test oracles"* — the permanent test is an invariant checked across scenarios, not a list of cases |
| S6 | this repository: `AGENTS.md` ("every play answers exactly one of `Played`/`Refused`"; "the two backends answer identically"), iteration 117's `sounds::cannot_play` and `needs-decision/undecodable-sound.md` | the design this extends: refuse at the play when the answer is already known; the cause in the log; no new `Refusal` variant |
| S7 | `rodio-0.22.2/src/decoder/mod.rs:358-388` (`Decoder::new`) and the mixer's own construction (`src/mixer/mod.rs:168`, `Decoder::new(Cursor::new(bytes.to_vec()))`) | the mixer decodes with `Decoder::new`, not with Bevy's `with_byte_len` builder, so Bevy's verdict is **not** reused for it (the two constructions may disagree on a file; measured below) |

## The technique

**Ask the backend that will decode, before the voice exists.** `AudioOutput` gains `decode_error(&self, bytes: &Arc<[u8]>) -> Option<String>` — **a defaulted method** (`None`: a fake output, which decodes nothing, is unaffected), overridden by `MixerBackend` with its own decoder construction (`Decoder::new(Cursor<Arc<[u8]>>)` — **no copy**; `play` copies the file with `to_vec()` on every play and the check must not). `play_requests` asks it when the mixer owns the voice and the bytes are loaded, **memoised per buffer** in iteration 117's cache in `SirenSounds`, and refuses at the play exactly as it does for Bevy's path: no `Played`, no voice, the decoder's words and the hint in the log. A backend whose `decode_error` says `None` and whose `play` then refuses is **lying about the contract**, and the crate's `start_pending` keeps answering for it as before.

## Parameters

| thing | value | source |
|---|---|---|
| the hook | `fn decode_error(&self, bytes: &Arc<[u8]>) -> Option<String> { None }` on `AudioOutput` | design default (an additive, defaulted method; `needs-decision/audio-output-decode-error.md`) |
| the mixer's check | `rodio::Decoder::new(Cursor::new(Arc::clone(bytes)))`, `Err` → its `Display` | S7, the construction `play` makes, minus the copy |
| when it runs | the first time a handle's bytes are seen at a play on the mixer path; **once per `(path, buffer)`** (the cache key gains *which backend asked*) | design default, iteration 117's key |
| the invariant | **a `Played` names an entity that exists when the frame ends** | S3, S4 |

## What good looks like

A bad file on the crate's own mixer: the first play is `Refused { MissingSound }` **and nothing else** — no `Played`, no entity, a log sentence with the decoder's words — and a game that hangs data on every `Played::entity` runs on without a panic. A good file is `Played`, started on the mixer, and its entity is alive at the end of that frame. In every scenario the crate can build — a normal play, a stolen voice, a refused file on either backend, a failed load — **no `Played` entity is dead at the end of the frame it was announced in.**

## Predictions (for the fix; written before it was built)

- **P1.** With a fake output whose `decode_error` answers, the bad bytes are refused at the play: 0 `Played`, one `Refused { MissingSound }`, 0 voices, the output's `play` never called, and the tagging game system does not panic.
- **P2.** The real `MixerBackend`'s `decode_error` agrees with its `play` on every file of the probe set (a Git LFS pointer, an empty file, a cut Ogg, an Ogg Opus, an ADPCM and a µ-law WAV, three good WAVs, two Oggs, the MP3 and the FLAC): *`decode_error` is `None` exactly when `play` returns `Some`.* **And Bevy's builder and the mixer's `Decoder::new` agree on all of them** (65 % — if they disagree on a file, that file is the reason the verdict is not shared).
- **P3.** Cost: the memoised check ≤ 30 ns a play; the first sight of a sound on the mixer path costs one decoder construction (a Vorbis 0.6 – 0.8 ms) *in addition to* the one `play` makes, once per sound.
- **P4.** **No existing test is edited**: the fakes in `tests/mixer.rs`, `tests/failed_load.rs` and `examples/budget.rs` inherit `decode_error`'s default and are untouched.
- **P5.** The invariant test (`Played` ⇒ the entity exists at the end of the frame) **fails on the unchanged tree for exactly the mixer-refused scenario** and for no other.
