# Music: layers that change on the bar

Adaptive music is not a second track. It is the same performance with each instrument's level keyed to game state, so
a fight can start without the music stopping: the tempo, the beat and the musicians are unchanged, and one part gets
louder.

```rust
use bevy_siren::*;

let score = MusicScore {
    layers: vec![
        MusicLayer::new("calm", "track.calm").at_db(-3.0),
        MusicLayer::new("combat", "track.combat"),
    ],
    tempo_bpm: 100.0,
    beats_per_bar: 4.0,
};
```

The plugin starts one looping voice per layer, and the layer at `SILENT_DB` stays in the mix. It costs one silent
voice and it keeps every layer phase-aligned — which is the whole point: an entry that starts from the first beat
sounds like a second song, and one that is already playing sounds like the same band getting louder.

## Moving the layers

```rust
// Bring combat in and calm out, on the next bar.
layers.write(MusicLayers::new(vec![
    ("combat".to_owned(), -3.0),
    ("calm".to_owned(), SILENT_DB),
]).next_bar());

// Or at once, for a cutscene that does not care about the beat.
layers.write(MusicLayers::new(vec![("calm".to_owned(), -12.0)]).now());
```

`fade_s` is **how long the move takes** — the time to arrive, like a duck's `fade_s`. `0` (the default) is the
crate's ~150 ms easing; a crossfade a game asks for is usually a bar or part of one:

```rust
layers.write(MusicLayers::new(vec![("combat".to_owned(), -3.0)]).next_bar().fade_s(2.4));
```

Each layer carries the fade its own change gave it, so three layers moved with three durations each take theirs. A
second change arriving mid-fade replaces the target and the fade — the layer does not queue — which is worth knowing
when a game's state machine can flicker.

**A target naming a layer the score does not have is named in the log, and the rest of the change still applies.** A
typo is the ordinary case — `cmobat` for `combat` — and the sentence says which layer and which score:
``refused `cmobat`: this score has no layer `cmobat`, so the change names nothing``. One bad target does not cost the
others: a message naming `combat` and `cmobat` moves `combat` and refuses `cmobat`. The entry is a `Refused`, so a busy
scene cannot scroll it away behind routine lines.

`When::NextBar` quantises to the next boundary from the tempo alone (`model::next_boundary`) — no beat map, no authoring
tool, and a frame that lands exactly on the bar is *on* it rather than past it. With `tempo_bpm: 0.0` there is no grid,
and every transition is immediate: a pad, not a song.

**`beats_per_bar` is four whether the score comes from a file or from `MusicScore::default()`.** A file that omits it
gets `MusicScore::default_beats_per_bar()`; so does `Default`, which is written by hand so the two cannot drift — a
`derive(Default)` would give the field `0.0`, which `bar_s()` reads as **no grid at all** and `problems()` reports. Note
that `tempo_bpm` and `beats_per_bar` treat zero differently on purpose: a tempo of zero means *"a pad, not a song"*, while
four beats is the only sensible bar and zero of them is a broken one.

A layer's `gain_db` is the layer's own place in the mix, and it **adds** to its cue's `gain_db`: the cue's is the
recording's trim (two masters, two levels), the layer's is what gameplay moves. Keep the level on the layer so a log
line reads the number a person set.

Gains ease at `MIX_TAU_S` (50 ms), so a layer moves without a click. A layer moved to `SILENT_DB` is not stopped; it is
silent and still playing, which is what makes bringing it back in the same performance.

## Stingers

```rust
stingers.write(Stinger::new("music.stinger").duck_db(-9.0).duck_s(1.5));
```

A stinger that **cannot** play ducks nothing: the music keeps its level, and the refusal is the only thing said about
it. The duck is armed when the play is answered, not when the message is written, so a missing sound or an unknown cue
leaves the mix where it was.

**A `duck_db` of zero, or a positive one, dips nothing and logs nothing.** The envelope is inert for any `by_db >= 0.0`
(`model::active_duck_db`), so the stinger plays its hit with the music untouched — and no `Ducked` line claims otherwise,
because a `Ducked` line means the bus moved. A raise is a lift, and a lift is not a duck.

A stinger plays once, **now** — a punctuation mark that waited for the bar would miss the moment — and ducks the
`Music` bus while it runs. It must be a `Music`-bus cue; one that names another bus is refused with a sentence in the
log rather than played somewhere unexpected.

`duck_s` is a **hold**, not a fade: the music stays at `duck_db` for as long as the stinger sounds, then releases over
`DEFAULT_FADE_S` (0.12 s, the same release every stop and every steal uses). The difference is audible — before
iteration 3 the bus began climbing on the frame the stinger fired, so a −9 dB stinger read −5 dB at its midpoint and
the music swelled back underneath it.

## Editing the score

A `MusicScore` is a resource, so a project (or the Cauldron) may change it while it plays. The layers are **voices**, and
they follow the score:

- **adding** a layer starts its stem;
- **removing** one fades its stem over `DEFAULT_FADE_S` and logs `layer removed` — an ownerless loop would otherwise play
  for the life of the process;
- **retargeting** one's `cue` fades the old stem and starts the new, logging `layer retargeted`;
- **changing a gain** moves the voice, which is what `MusicLayers` does — no restart.

`MusicState::names()` is in score order, whatever the edit.

## When a stem's voice dies

A layer is a **voice**, and a voice can be stopped by something other than the score. The plugin reconciles its mirror
every frame, so `voice(name)` never names an entity the world no longer has and `live()` is true:

- **the budget took it** — a stem is never the victim: a music-layer voice is unstealable whatever priority its cue
  authored. `UNSTEALABLE_PRIORITY` is a label a cue carries, and a layer is a stem; a play that needs room on the music
  bus is refused instead of taking one. If a voice dies anyway, it is asked for again at the layer's own gain.
- **the game stopped it** — `StopCue` is a decision the crate was told to make. The layer stays stopped, `voice(name)` is
  `None`, and `layer_db(name)` reads `SILENT_DB`. A **retarget** re-arms it: the score names a different sound.
- **its first play was refused** — the intent to start is retried when a voice frees, not every frame: the retry waits
  for the live count to fall below the count at the refusal, so a full budget writes **one** refusal line, not one per
  frame.

## A gain that is not a number

A layer's `gain_db` — in the score and in a `MusicLayers` move — follows the crate's one rule for a number that is not a
number: it is the **neutral** value, `0.0` dB, never silence. `SILENT_DB` is a **finite** number, so a score that starts
a layer silent still starts it silent; only a value that is not a number at all is normalised. Without this, `NaN` went
straight into `layer_db`, `target_db` and the voice's `base_db`: the sink was silenced by `chain_db` (correctly) while
the fader reported `NaN`, and `MusicScore::problems` said nothing.

`MusicScore::problems` now names it in one sentence — the score's report is `Vec<String>`, unlike the cue table's
`Vec<CueProblem>`, so this added no public variant:

```
music layer `calm` has a gain that is not a number
```

## A layer must name a cue that loops

A layer is a stem: it runs throughout, at the same tempo and beat as every other layer. A cue with `Mode::Once` **ends**,
so it cannot hold a layer — the voice would be retired, the layer would be asked for again, and the result is a
one-shot at frame rate, not music. The plugin **does not start** such a layer, keeps it in the score's `names()`, and
writes **one** line:

```
refused `hit`: music layer `calm` names cue `hit`, which is not a loop, so it cannot hold a layer
```

That is `MusicScore::problems`' sentence, so the report and the log agree. Fix the cue to `Mode::Loop` and the layer
starts on the next frame — the plugin reads the cue the layer names *this frame*, not the one it had when the score was
loaded. `needs-decision/not-a-loop-refusal.md` records why this is a log line rather than a new `Refusal`.

A stem that comes back starts from the beginning, so it is out of phase with the others. That is the price of a voice
that died; the crate does not re-time it, because what a restart means after a gain change is a decision the score does
not carry.

## A stem's file: how long a loop is

A loop is as long as its file says, and the format decides how far that is from the length of the sound. The crate does not decode a stem to measure it: it reads the length the container declares and,
where the decoder returns more than that, cuts the extra at the loop point — **on Bevy's path and on the `mixer` feature's own graph alike**. A stem that is a block too long is late by that block on every
pass, and two stems that are late by different amounts are no longer on the same bar.

| Format | A loop's length | Why |
|---|---|---|
| WAV, FLAC | exact | the container counts the samples (measured: `tone.flac` is 24 000 samples a pass) |
| MP3 | not cut | `tests/fixtures/tone.mp3` carries an encoder tag (`Info`, `LAME4.0`, which `ffmpeg` writes) and loops at exactly its 24 000 samples; **an MP3 with no tag was not measured**, and the crate does not read one |
| Ogg Vorbis | **cut to the last page's granule position** | the decoders return the padding of the last block — on Kenney's 254 CC0 Ogg effects, 51 files by 27 to 936 frames (a median of 9 ms, the loudest at −6 dBFS) — so `tone.ogg` looped at 24 576 frames and now loops at its 24 000: a 12 ms gap on every pass is gone |

An Ogg the crate cannot read one length from — a chained or grouped file, a file cut short, a stream whose packet count cannot account for its last granule position — is looped as the decoder returns
it. A stream that starts at a non-zero granule position (Vorbis I allows one: *"a positive starting value simply indicates that this stream begins at some positive time offset"*) is not detected; its loop is no worse than it was.

**An Ogg loop needs a Vorbis decoder whose loop is not empty.** `rodio`'s `symphonia` Vorbis decoder reports an empty first span, and `repeat_infinite` — Bevy's own loop and the mixer's — of it produces **no samples**
(RustAudio/rodio#775; Bevy's 0.18 → 0.19 migration guide: *"OGG/VORBIS support through `symphonia` is currently subject to issues with buffering, reverb, looping and spatial audio"*). This crate's
`Cargo.toml` selects `lewton`, which is Bevy's own default, and the `mixer` feature's loops are protected from the other decoder; if a game turns on Bevy's `symphonia-vorbis` itself, an Ogg stem on
**Bevy's** path is silent whatever this crate does, and a WAV, FLAC or MP3 stem is unaffected.

## As data

```ron
(
    layers: [
        (name: "calm", cue: "track.calm", gain_db: -3.0),
        (name: "combat", cue: "track.combat"),
    ],
    tempo_bpm: 100.0,
    beats_per_bar: 4.0,
)
```

`MusicScore::problems(&table)` names a layer with no name, the same layer twice, a layer naming a cue the table does
not have, a layer whose cue is not a loop (a layer that ends is not a layer), a tempo that is negative or not a
number, and a score with no layers.

## Reading it back

`MusicState` is the plugin's own record, for a panel or a test: `layer_db(name)` is where a layer is now,
`target_db(name)` where it is going, `voice(name)` the entity playing it, `moving()` how many gains are still easing,
and `next_change_s()` the moment the next queued change is due.

## The example

```sh
cargo run --example music
```

A scripted calm → combat → calm, printing each transition and checking that every one landed on a bar (2.4 s at
100 bpm).

## A layer with no voice yet

A score can name more layers than the budget has room for. When `Play` is refused, the layer is a **virtual voice** —
Gregory §14.5.8 (*Game Engine Architecture*, 3rd ed.): *"a sound the game believes is playing but that has no hardware
voice"*. The plugin records the live count at the refusal and asks again when that count falls below it, rather than
retrying every frame into a log a panel reads.

`MusicState::blocked(layer)` is that state, for a panel:

```rust
let waiting = music.blocked("combat");   // Some(1): refused while one voice was live
```

`None` means the layer was never refused. A panel that draws only `layer_db` cannot tell a waiting layer from one
authored silent — both read `-80.0 dB` — which is why `examples/mixer` appends `waiting` to the gain.
