# `AudioOutput::decode_error`: asking the backend whether it can decode, before a voice exists

*Iteration 118, 2026-11-04. A public trait gains a method; additive, so built under the standing "recommend, record, proceed" rule (`assumed-decision`).*

## What changed in the public surface

`bevy_siren::output::AudioOutput` — the trait the crate's own mixer implements and a test's fake output implements — gains one method with a default body:

```rust
fn decode_error(&self, _bytes: &Arc<[u8]>) -> Option<String> { None }
```

`None` means *this backend decodes these bytes* (or decodes nothing — a fake). `Some(words)` means *`play` would return `None` for them*. `MixerBackend` overrides it with the decoder construction its `play` makes (`rodio::Decoder::new(Cursor::new(Arc::clone(bytes)))`) and returns the error's `Display`. No type, field, message or `Refusal` variant is added or changed; the trait's doc already says a game never implements it.

## Why

On the crate's own mixer a play of bytes `rodio` refuses was answered `Played` (in `play_requests`) and then `Refused` (in `start_pending`, the next system), with the voice entity despawned **in the frame it was announced**. A game that hangs data on `Played::entity` — the use the field documents — panicked in Bevy's command error handler. The README, the crate docs and `AGENTS.md` all say both backends answer identically; Bevy's path already refused such bytes at the play (iteration 117). The crate cannot decode with `rodio` itself in the default build (the closed dependency list: `rodio` is the `mixer` feature's), so the backend that owns the decoder is asked.

## Options

| # | option | cost to a consumer | verdict |
|---|---|---|---|
| 1 | **A defaulted method on `AudioOutput`** (this change) | none: a fake that implements the trait compiles unchanged and gets `None`; the six fakes with placeholder bytes in `tests/mixer.rs`, `tests/failed_load.rs` and `examples/budget.rs` are untouched | **chosen** |
| 2 | A new trait (`Decodes`) the output also implements | a second trait object or a downcast; `Output` would hold two boxes | more surface for the same answer |
| 3 | Make `Output` generic over a decoder, or add `rodio` to the default build to decode in `play_requests` | a new dependency in every consumer's default build; the closed list in `tests/leaf.rs` | no |
| 4 | Keep `start_pending` as the only decider and make the refusal quiet (`try_insert` is the game's business) | a documented use that panics; a rule (*one answer per play*) that is false | no |
| 5 | A required method | breaks every implementor, including the crate's own tests | no |

## Edge cases and what is decided

- **A backend whose `play` can refuse must override `decode_error`.** One that does not keeps the old two answers, which `start_pending` still gives (`tests/mixer.rs::a_sample_the_decoder_refuses_names_its_sound_key` holds it). The trait's doc says so.
- **Bytes that are not loaded yet** are not asked: the voice is spawned and waits, as before, and `start_pending` refuses it in the frame the bytes arrive (`Played` could not have known).
- **One question per buffer.** `SirenSounds` keeps one memo per asker (Bevy's decoder; the output), keyed by buffer address, length and first 16 bytes; an edit is asked again. Measured: a hit 20.7 ns; the first sight of a Vorbis 0.63 – 0.66 ms (one decoder construction, in addition to the one `play` makes), MP3 10 µs, FLAC 22 µs, WAV 7 – 21 µs.
- **The two decoders agreed on 14 of 14 files** (Bevy's builder with `with_byte_len`, and `Decoder::new`), words included; they are still asked separately, because an agreement is a measurement and not a contract.
- **No new `Refusal` variant.** `needs-decision/undecodable-sound.md`'s option 4 stands: `MissingSound { sound }` carries the key and the log carries the decoder's words and the hint.

## Recommendation

Option 1, as built. Revisit only if a second backend (a different mixer, a middleware) wants to say *why* in a structured way — then `decode_error`'s `Option<String>` is the thing to widen, in one place.
