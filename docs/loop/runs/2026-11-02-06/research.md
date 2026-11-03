# research.md — iteration 6: a refused sample names the sound, not the cue

## The item

`playback::start_pending` starts a voice on the crate's own mixer once its bytes are loaded. When `AudioOutput::play`
returns `None` — the sample cannot be decoded — it reports:

```rust
let reason = Refusal::MissingSound { sound: voice.cue.clone() };
```

`voice.cue` is the **cue's** name (`footstep.grass`); `MissingSound::sound` is documented as *"the key with no handle"*
(`step_03`). So a decode failure tells a project to go looking for an asset key that does not exist, in a list of
keys that never contained it:

```
no handle for sound `footstep.grass`
```

when the true statement is that `step_03.wav` is present, was mapped, and the **decoder rejected its bytes** —
a truncated file, a format the build's `rodio` features do not include, or a `.wav` header that lies about its length.
A project searches its `audio/` folder for a file called `footstep.grass`, finds nothing (because the cue was never a
file), and has been told something false by the one part of the crate that exists to say what is missing.

Found in iteration 5 by the `examples/budget` probe, whose dummy four-zero-byte samples tripped exactly this path.

## Sources

1. **Bevy 0.20's own asset contract**, vendored at
   `~/.cargo/registry/src/index.crates.io-…/bevy_asset-0.20.1/src/server/mod.rs:2114`:

   ```rust
   pub enum LoadState {
       NotLoaded, Loading, Loaded,
       /// The asset failed to load. The underlying [`AssetLoadError`] is referenced by [`Arc`] clones …
       Failed(Arc<AssetLoadError>),
   }
   ```

   Bevy distinguishes **a file that was never found** from **bytes that failed to load**, and carries the error in the
   second case. `bevy_audio`'s `AudioSource { bytes: Arc<[u8]> }` (`bevy_audio-0.20.1/src/audio_source.rs`) is opaque
   bytes — the crate can tell that they are undecodable only by asking the decoder, which is what `play` does — but the
   two failures are still two facts and Bevy's own model says so.

2. **`rodio` 0.22.2**, vendored: `rodio::Decoder::new(Cursor::new(bytes))` returns
   `Result<Decoder<_>, DecoderError>` and `MixerBackend::play` turns the `Err` into `None`
   (`src/mixer/mod.rs:147`). The decoder's error is the evidence a person needs; the crate has it in hand on that line
   and discards it.

3. **The crate's own rule R6** (`docs/research/siren.md`): *"a play is never silent: `Played` or `Refused`, both with the
   reason."* And the loop's own standard: *"a refusal is the most valuable bug of all, because it is silent … the
   `Refusal` variants … each names what is missing."* A reason that names the **wrong** thing is the failure mode this
   crate's refusals exist to prevent.

4. **`MissingSound`'s own doc and its one other site.** `answer` produces it when `SirenSounds::get(&slot.sound)` is
   `None` — genuinely a missing *handle* — and the sentence is `"no handle for sound \`{sound}\`"`. The decode path
   borrowed the variant and the sentence without either being true.

5. **`examples/budget`'s probe, iteration 5**, which is the reproduction: a `SirenSounds` with real keys and four-zero-
   byte assets produced `no handle for sound \`line\`` for a cue called `line` whose key was also `line` and was
   present. That coincidence hid the bug from every test.

## The technique

Two facts, two sentences, no new public type — because the **key is derivable** and the variant's `sound` field already
carries a key:

- `start_pending` has `voice.cue`, `voice.variant`, and the `CueTable`. The key is
  `table.get(&voice.cue)?.sounds.get(voice.variant)?.sound` — the same expression `answer` used to choose it. A lookup
  that fails (a table edited between the play and the load) falls back to the cue name only as a last resort, and
  says so.

So the fix has two halves, and only one of them is a sentence change:

| half | what it does | API |
|---|---|---|
| **key** | the refusal names the sound the decoder rejected, not the cue | none |
| **sentence** | `MissingSound`'s line becomes true for both of its causes: *"no handle or undecodable bytes for sound `step_03`"* | none — the variant and its field are unchanged |

The doc on the variant changes to say what it now means: *"no handle for that sound, or bytes the decoder refused"* —
which is what the two call sites have always produced, one of them lying about it until now.

A **third** refinement is possible and goes to a decision file: a dedicated `Refusal::Undecodable { sound }`, so a host
can tell "you never mapped this key" from "this file is corrupt". That is a new variant (an exhaustive `match` in a
consumer breaks), so `needs-decision/undecodable-sound.md` carries it.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what the refusal names | the **sound key**, never the cue | `MissingSound::sound`'s doc; `answer`'s existing use |
| the two causes of one variant | no handle, or undecodable bytes | `answer` + `start_pending`, and Bevy's `LoadState::{Failed, …}` split |
| when the decode happens | once, when the bytes first load | `start_pending`'s `assets.get(&samples.0)` guard |
| the decoder's own error | available at `mixer/mod.rs:147`, discarded | `rodio::Decoder::new(..).ok()?` |

No new number, no new field on a serde type, no new threshold.

## What good sounds like

Nothing audible changes. What changes is the sentence a project reads when a file is broken:

| case | before | after |
|---|---|---|
| a key with no handle | ``no handle for sound `step_09` `` | unchanged (and its test still passes) |
| a key whose bytes do not decode | ``no handle for sound `footstep.grass` `` | ``no handle or undecodable bytes for sound `step_03` `` |
| the cue left the table before its bytes loaded | ``no handle for sound `footstep.grass` `` | names the cue, and says the table no longer has it |

A reviewer judges it without hearing anything: the budget probe's `refusals.txt` will carry both causes side by side —
a key with no handle and a key whose bytes are four zeroes — and the two lines must name **different keys**, where today
both name cues. That pair is the evidence, and either line alone would pass vacuously.
