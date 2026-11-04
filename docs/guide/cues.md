# Cues: a sound, named once

A **cue** is the thing gameplay asks for. Not a file: `"footstep.grass"` is a name with several recordings behind it,
a bus, a level, a pitch range, a repeat rule and a voice limit. Gameplay says what happened; the crate decides what
that sounds like.

```rust
use bevy_siren::*;

let mut table = CueTable::new(vec![
    Cue {
        sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02"), SoundSlot::new("step_03")],
        gain_db: -4.0,
        pitch: PitchRange { low: -1.5, high: 1.5 },
        repeat: Repeat::AvoidRepeat { avoid: 2 },
        max_instances: 2,
        priority: 10,
        ..Cue::new("footstep.grass")
    },
    Cue {
        sounds: vec![SoundSlot::new("burst_01")],
        repeat: Repeat::Shuffle,
        bus: "Effects".to_owned(),
        ..Cue::new("gun.burst")
    },
]);
```

Ask for it with a message:

```rust
play.write(Play::cue("footstep.grass"));
play.write(Play::cue("footstep.grass").at(position));   // a position, for a spatial cue
play.write(Play::cue("ui.cancel").gain(-3.0));          // this play only
```

Everything a play decides comes back as one `Played`:

```rust
for answer in played.read() {
    // answer.sound is the recording; answer.bus the bus; answer.gain_db and answer.pitch the level and detune.
    // answer.entity is the voice, for StopVoice or for your own data.
}
```

## When it cannot play, it says why

There is no silent failure. Every refused play is a `Refused` with a reason that names what is missing:

| Reason | What was missing |
|---|---|
| `UnknownCue` | no cue of that name |
| `NoSounds` | the cue has no variants |
| `MissingSound { sound }` | no handle for that `sound` key in `SirenSounds` |
| `UnknownBus { bus }` | the cue names a bus the mix table does not have |
| `VoiceLimit { bus }` | the budget could not be made room in |
| `SpatiallyMute(position)` | `Play::at(..)` with no `SpatialListener` in the world |

A refusal names what a project has to supply. It never falls back to another bus or a guessed file, because a
sound that plays the wrong thing is worse than one that says what is missing.

## The keys and the files

A cue's `sound` is an opaque key. `SirenSounds` maps keys to loaded handles:

```rust
sounds.insert("step_01", assets.load("audio/step_01.ogg"));
// or, when a key and a file share one name under one directory:
let asked = sounds.load_table(&assets, &table, "audio");
```

Nothing in the crate opens a path it guessed. A key is data, a file is the project's business, and the mapping is one
place — so a delivered recording replaces another without touching the table, and a cue table can be written before an
artist delivers anything.

`sounds.missing(&table)` is every key still unanswered (an editor shows it), and `sounds.failed(&assets_server, &table)`
is every key whose load the server reported as failed — **a list you no longer have to poll to learn a play will be
silent**: the crate asks the server itself, and a play of a failed file is refused (below).

**A reload must call `retain_keys` first.** `load_table` **merges**: it asks for the keys it has no handle for and never
drops one the table has lost, and `SirenSounds` has no other remover. That matters because a handle is what keeps an
`AudioSource` alive, and an `AudioSource` is the **whole encoded file** — so a project that reloads a table with renamed
keys kept one audio file resident per key it had ever loaded:

```rust
sounds.retain_keys(&new_table);                 // drop what the new table does not name
let asked = sounds.load_table(&assets, &new_table, "audio");
```

Measured: two tables of ten different keys loaded in turn left **twenty** keys held. `sounds.len()` is the number of keys
the table names once a reload retains.

## Checking a table

`CueTable::problems()` returns `CueProblem`s in sentences a person reads on a status line: a cue with no sounds, a
duplicate name, a bus the mix table does not have, an inverted pitch range, an attenuation constant that cannot
produce a gain, and `max_instances: 0` (no per-cue limit, which is almost never meant). Call it once at load, and
print what it says.

**A gain that is not a number is neutralised, and the log says so.** `gain_db` on a cue or on a sound slot is read as
**no gain** at play time — the same rule `distance_gain` follows for a distance — so the cue sounds at the level it
would have had and `Played.gain_db` reports that number (iteration 18). The behaviour is right; what was missing was
the *report*, and since iteration 72 it is in `SirenLog` the moment the play is answered:

> `2.00s stopped `footstep.grass`: played, but cue gain_db and sound gain_db were not a number and each was read as the
> neutral value`

It arrives through `Entry::Stopped`'s free-text `why`, so **no public enum gains a variant** and a host that matches
`CueProblem` exhaustively is unaffected. That was the choice `docs/loop/needs-decision/bad-gain-problem.md` recommended:
a `CueProblem` variant is still the tidier home and still costs a breaking change, so it waits for a minor release.

An inverted pitch range and a negative attenuation constant are still reported by `problems()`, because those have
variants already. A `gain_db` you computed is still worth checking for `is_finite` before it reaches the table — the log
tells you after the fact, and the table check tells you before.

```rust
for problem in table.problems_in(&settings) {
    eprintln!("siren: {problem}");
}
```

**Call `problems_in`, not `problems`, when the project lists its own buses.** `problems()` is a wrapper over
`problems_with(&DEFAULT_BUSES)`: it checks against the crate's default four names, so in a project with a `Sfx` or
`Voices` bus every cue on one is reported as `UnknownBus` — while the crate accepts and mixes it, because `answer`'s
`UnknownBus` check reads `settings.bus_names()`. `problems_in(&settings)` asks that same question. A host with no settings
at hand (a test, a data check) can still call `problems()` for the default-four answer.

## As data

With `serde` on, a table is a RON file:

```ron
(
    cues: [
        (name: "footstep.grass", sounds: [(sound: "step_01"), (sound: "step_02")], gain_db: -4.0),
        (name: "wind", sounds: [(sound: "wind_loop")], bus: "Ambience", mode: Loop),
    ],
)
```

Every field but `name` has a default, so a hand-written file needs only what a person cared about; `deny_unknown_fields`
means a typo is an error and not a silently ignored line. `CueTable::from_ron` and `to_ron` are the pair, and the
round trip is a test.

## Repeat rules

A `MissingSound` names the **sound key** a project maps to a file, never the cue — for all three of its causes: a key with
no handle, a key whose bytes the decoder refused, and **a key whose file failed to load**. ``no handle or undecodable bytes
for sound `step_03` `` sends you to `audio/step_03.wav`, which is the file to fix.

**A file that failed to load is refused, with the cause in the log.** Type `step_3.wav` for `step_03.wav` and the asset
server knows the load failed and says why; the crate asks it where it decides, so the first play reads

```
0.15s refused `footstep.grass`: the file for sound `step_3.wav` failed to load: Path not found: /…/assets/audio/step_3.wav
```

and writes no `Played`, spawns no voice and holds no slot (before this, every play of that file was answered `Played`,
nothing ever sounded, and the log showed a healthy *played* and *stolen by footstep.grass (cue)*). A voice spawned while
the load was still in flight, whose load then fails, is refused in the frame the server says so — after the `Played` it
could not have known to withhold — and despawned. **A load still in flight is not refused**: it waits, as it always did,
and nothing is invented about a slow disk. The `Refused` message carries the one variant, so a game that prints
`reason` reads the one sentence; the log has the cause (`docs/loop/needs-decision/undecodable-sound.md` weighs a variant
of its own).

**Bytes that load and cannot be decoded are refused too — on Bevy's path and on the crate's own mixer's.** A file can load (the asset loader returns the bytes and stops) and not be audio the build can decode: a **Git LFS
pointer** where the audio should be, a file with no bytes, a cut one, an **Ogg Opus** file in a `.ogg`, an ADPCM WAV. Bevy's own `AudioSource::decoder` is `….build().unwrap()`, so on the default build that file used to be `Played`
and then **panic the game in the same frame**, at `bevy_audio-…/src/audio_source.rs:101`, naming no file; on the crate's own mixer it was `Played` and then `Refused` **in the same frame**, with the voice despawned in it, so a
game that hung data on `Played::entity` — the use that field documents — panicked in Bevy's command handler instead. The crate now asks **whoever will decode the bytes**, once per asset, before a voice exists: Bevy's call on Bevy's path,
the mixer's own decoder (`AudioOutput::decode_error`) on the crate's own mixer:

```
0.05s refused `ui.click`: the bytes of sound `click.ogg` cannot be decoded: The format of the data has not been recognized. — a Git LFS pointer, not the audio file
```

No `Played`, no voice, no slot. The decoder's own words are the same for every one of these files, so the three commonest are told apart by their first bytes and the sentence says which (a hint changes a sentence, never a
decision). A voice spawned while the bytes were still loading is refused in the frame they arrive — after the `Played` it could not have known to withhold — and despawned, before Bevy's `PostUpdate` can decode it, or by the mixer's
own `start_pending`. **What is decoded is a property of the build**: an MP3 or a FLAC is refused where `rodio` has no decoder for it and played where it has one (`mixer`, `semantic`, or Bevy's own `mp3`/`flac`), because the check
is the decoder's own answer and not a list of formats.

`Repeat::AvoidRepeat { avoid: n }` refuses the last `n` variants, and the default `avoid: 1` is what stops the
machine-gun footstep. **The window is the number the cue asks for, read on every play** — a table an editor reloads with
a larger `avoid` takes effect on that cue's next play, with nothing to reset. (Before this fix a cue's picker was sized on
its first play and kept that window for the app's life, so `avoid: 3` over four sounds produced the byte-identical
sequence to `avoid: 1`.) When every variant is inside the window — two sounds and `avoid: 4` — the pick immediately
before is still refused, because repeating it is the one thing the window exists to prevent. `Repeat::Shuffle` is a
plain draw, repeats included: a burst weapon wants one sample and no variation.

**The window is made of *sounds*, not of slot positions.** A cue that lists one file twice — a copy-pasted line, a table
generated from a folder scan — has two slots and one sound, and `AvoidRepeat` refuses both slots once that sound has been
heard. So `sounds: [(sound: "a"), (sound: "a"), (sound: "b")]` always alternates, where it used to be able to play `a`
three times running; a cue of `[(sound: "a")]` alone still repeats, because the crate cannot invent a recording. If a
cue is meant to have two recordings, deliver both — `problems()` does not yet report a duplicate, and
`docs/loop/needs-decision/duplicate-sound-problem.md` is where that waits.

## Stopping a sound

`StopVoice` takes the entity `Played` reported and fades it over `DEFAULT_FADE_S`; `StopCue` stops every voice of a cue
by name. **A stop for a sound that has already ended is a no-op** — a one-shot is retired the frame its sample ends, so
a game holding a handle it got a moment ago is racing the sound's own length, not making a mistake. Nothing is logged
for it: the log holds decisions about sounds, and this is a sound that had already stopped. A second stop for a voice
already fading is likewise skipped, so a fade is begun once.

## The seed

Every variant pick and every detune comes from one stream, and **that stream's seed is `SirenSettings::seed`** — the
project's file, not a crate constant. Two projects that differ only in `seed` play different variants and different
detunes from the first play; the same number plays the same sequence twice, field for field, in two processes. That is
what makes a capture, a replay and an A/B of an audio change reproducible.

Changing the seed while the game runs **replaces** the stream and every variant history rather than continuing from
the old position: the next play is the new seed's first draw. The seed is configuration, so the change is not written
to `SirenLog` — the log is for what happened to a sound.

**The stream is shared, so a play's draws move every later play's.** A play draws once for its variant and once for its
detune — the detune **even when the cue's `pitch` range is exact**, because `answer` computes
`cue.pitch.at(choices.rng.next_f32())` before it knows the range cannot use the number. So adding a fixed-pitch cue to a
scene shifts every later pick for every other cue that shares the stream: a project's soundscape is a function of its
seed and its **play order**, not its seed alone. Measured in
`tests/play.rs::an_exact_pitch_range_still_draws_from_the_stream`. Skipping that draw is
`docs/loop/needs-decision/exact-pitch-draw.md`, and it is not a patch because it would re-roll every project's
soundscape in the build that shipped it.

## A cue's history, and what the table owns

Each cue name gets one `VariantPicker`, created on that cue's first play, and it holds the window of recent sounds that
`Repeat::AvoidRepeat` refuses. Two methods on `SirenChoices` manage it, and they do different things:

- **`reset_histories()`** empties every picker but keeps every entry — the next play of each cue is unconstrained, and
  the map is the same size.
- **`retain_cues(&table)`** drops the pickers of cues the table no longer has. `play_requests` calls it itself, and only
  on a frame where the table changed.

That second one matters for a project that **generates cue names** — one per level, per enemy, per dialogue line — or
that reloads a table with different names. Without it, one picker per name ever played lived for the life of the
process, each holding an owned copy of its recent sound keys. Measured: 200 names played and the table cut to ten still
held **200**; `SirenChoices::picker_count()` is the number if you want to draw it.

**`0` is not a stream of its own.** xorshift's state must never be zero, so `Rng::new(0)` maps it to
`model::Rng::ZERO_SEED` — which is exactly `SirenSettings::default_seed()`. A project that writes `seed: 0` therefore
hears the crate's **default** stream, and `SirenChoices::seed()` reports `ZERO_SEED`, not `0`, because that is the seed
the stream was built from. If you want a stream unlike the default, write any other number.
