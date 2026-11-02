# The Cauldron: `<project>/family/bevy_siren.ron`

`bevy_grimoire`'s audio panel hosts this crate as an optional family feature, and the file it edits is **this crate's
own types** — never a second schema:

```ron
(
    settings: (
        master_db: 0.0,
        max_voices: 32,
        buses: [
            (name: "Music", gain_db: -2.0),
            (name: "Effects", max_voices: 16),
            (name: "Ambience", gain_db: -6.0),
            (name: "Dialogue", max_voices: 2),
        ],
        ducks: [
            (source: "Dialogue", target: "Music", by_db: -8.0, hold_s: 0.25, fade_s: 0.6),
        ],
    ),
    cues: (
        cues: [
            (name: "footstep.grass", sounds: [(sound: "step_01"), (sound: "step_02")], gain_db: -4.0),
        ],
    ),
    score: (
        layers: [(name: "calm", cue: "track.calm")],
        tempo_bpm: 100.0,
    ),
)
```

The panel writes `SirenSettings`, `CueTable` and `MusicScore` through their own `serde` derives, so the file a project
ships is the file a person edits, and a field the crate adds is a field the panel reads without a migration.

## What the panel reads

- Each bus's level from `SirenMix::db`, its duck from `ducked`, and its live voices from `voices` — the same numbers a
  game's own meter reads.
- The log tail from `SirenLog`, so "why is the music quiet" has an answer on screen.
- `SirenSettings::problems()` and `CueTable::problems()`, on the status line. A file that does not parse is a problem
  message, never a silent default.

## What the panel writes

- An edit to a bus, a duck, a cue or a layer: the same RON file, through `to_ron`.
- An **Audition** on the selected cue: `Play::cue(name)`, which the panel's own log then shows as `Played` or
  `Refused` — the panel hears through the same door a game does.
- A settings change is a `Resource` write, so the running editor's own mix follows the panel while it is open.

## The pin

The editor depends on this crate by git revision, never by path:

```toml
bevy_siren = { git = "https://github.com/bevy-coven/bevy_siren", rev = "<sha>", features = ["serde"], optional = true }
```

and a feature named after the crate, off by default, so a project that lists no family crate pays nothing.

## The two-way street

This crate is canonical: a change the editor needs is made **here**, in its own commit with its own test, requested as
an issue on this repository labelled `consumer` and `consumer:bevy_grimoire`. The editor never patches, wraps or vendors
a copy. The mirror image is in `bevy_grimoire`'s `.omp/commands/build-loop.md`: a sound need the editor cannot express
goes to this repository as an `S-NN` issue, and the editor item waits with `Needs: bevy_siren#N` until a commit here
closes it, then moves its `rev` pin.
