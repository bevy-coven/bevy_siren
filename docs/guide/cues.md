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
is every key whose load the server reported as failed.

## Checking a table

`CueTable::problems()` returns `CueProblem`s in sentences a person reads on a status line: a cue with no sounds, a
duplicate name, a bus the mix table does not have, an inverted pitch range, an attenuation constant that cannot
produce a gain, and `max_instances: 0` (no per-cue limit, which is almost never meant). Call it once at load, and
print what it says.

**A gain that is not a number is not in that list.** `gain_db` on a cue or on a sound slot is neutralised at play time —
`model::finite_or` reads it as **no gain**, so the cue sounds at the level it would have had and `Played.gain_db` reports
that number (iteration 18). The play is right and the *report* is silent, which is the one place these two kinds of
authoring mistake differ: an inverted pitch range and a bad attenuation constant in the same struct **are** reported,
and a non-finite gain is not. A seventh `CueProblem` would fix it, and `CueProblem` is a public enum without
`#[non_exhaustive]`, so adding it breaks every consumer's `match` — `docs/loop/needs-decision/bad-gain-problem.md` holds
the question. Until then: a `gain_db` you computed is worth checking for `is_finite` before it reaches the table.

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

A `MissingSound` names the **sound key** a project maps to a file, never the cue — for both of its causes, a key with
no handle and a key whose bytes the decoder refused. ``no handle or undecodable bytes for sound `step_03` `` sends you
to `audio/step_03.wav`, which is the file to fix.

`Repeat::AvoidRepeat { avoid: n }` refuses the last `n` variants, and the default `avoid: 1` is what stops the
machine-gun footstep. When every variant is inside the window — two sounds and `avoid: 4` — the pick immediately
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
