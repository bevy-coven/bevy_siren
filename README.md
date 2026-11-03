# bevy_siren

> ⚠️ **Vibe Coded.** Written by AI agents working from a human's direction, covered by tests rather than a line-by-line human review. Read the code before you rely on it for something you care about.

Sound for Bevy 0.20: a cue table as data, buses that duck, a voice budget that refuses politely, layered music
that changes on the bar, and a log of what played and why.

[![crates.io](https://img.shields.io/crates/v/bevy_siren.svg)](https://crates.io/crates/bevy_siren)
[![docs.rs](https://docs.rs/bevy_siren/badge.svg)](https://docs.rs/bevy_siren)
[![CI](https://github.com/bevy-coven/bevy_siren/actions/workflows/ci.yml/badge.svg)](https://github.com/bevy-coven/bevy_siren/actions/workflows/ci.yml)
[![Bevy](https://img.shields.io/badge/bevy-0.20-blue.svg)](https://bevyengine.org)
[![licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#who-and-licence)

![The audio panel, animated: four bus rows with their levels, ducks and live voice counts; a line of dialogue ducks Music by 8 dB and Effects by 4 dB; a third footstep steals a voice from a bus capped at two; the combat layer comes in on the bar. Below, the eleven-cue table — `SIREN_FAULTS=1` adds two deliberate mistakes, so two rows carry `<- problem` and the `Problems` block names them — and the log tail reporting each decision as it happens.](https://raw.githubusercontent.com/bevy-coven/bevy_siren/main/docs/media/mixer.gif)

*`examples/mixer`: a scripted run — a line of dialogue ducks the music, a third footstep steals a voice from a bus
capped at two, and the combat layer comes in on the bar.*

![The same panel at rest: Music at −9 dB with a −8 dB duck, Effects at −4 dB with one of two voices, Ambience at −6 dB, Dialogue at 0 dB, the nine shipped cues with their buses, modes, gains and priorities, `Problems` reading `none`, and the log tail](https://raw.githubusercontent.com/bevy-coven/bevy_siren/main/docs/media/mixer.png)

*The still: 1280 × 720.*

**Status:** usable · Bevy 0.20 · MSRV 1.97.1

**Why:** Bevy gives you an `AudioPlayer` and a mixer. What every game still rebuilds on top is the part that
decides *what* to play and *how loud*: a cue with several recordings so a footstep does not machine-gun, a bus
structure so dialogue steps the music back, a voice budget so a firefight does not spawn four hundred sinks, music
layered by instrument so combat starts without cutting the track, and a log that says why the music went quiet.
`bevy_siren` makes that one crate's worth of it shared.

## Quick start

```sh
cargo add bevy_siren
```

```rust,no_run
use bevy::prelude::*;
use bevy_siren::*;

fn main() {
    App::new().add_plugins((DefaultPlugins, SirenPlugin)).add_systems(Startup, setup).run();
}

fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
    // A cue with two variants: the picker never plays the same one twice in a row.
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

fn walk(mut play: MessageWriter<Play>) {
    play.write(Play::cue("footstep.grass"));
}

fn hear(mut played: MessageReader<Played>, mut log: ResMut<SirenLog>) {
    for answer in played.read() {
        // Which recording the player heard, and why it was chosen.
        log.push(Entry::Played {
            at_s: 0.0,
            cue: answer.cue.clone(),
            sound: answer.sound.clone(),
            bus: answer.bus.clone(),
            gain_db: answer.gain_db,
        });
    }
}
```

The README's quick start is the crate-root doctest (`tests/readme.rs` fails if the two drift), and
`cargo test --features readme-example --test all readme::` is the check.

From git, for changes not released yet:

```toml
[dependencies]
bevy_siren = { git = "https://github.com/bevy-coven/bevy_siren" }
```

Features: `serde` (the cue table, the mix table and the score as RON, with `from_ron`/`to_ron`) and `mixer` (the
crate's own summing graph on `rodio`, one number per bus per frame). Both answer `Played`, `Refused` and `Voice`
identically; only the sample path differs.

## Try it

```sh
cargo run --example mixer
```

The panel: every bus with its level, its duck and its live voice count, the cue table, and the log of the last
decisions. `SIREN_KEYS=steal,layer,duck` stages a run so a capture shows the three things this crate exists for.

```sh
cargo run --example music
```

A two-layer score following a scripted calm → combat → calm, printing each transition's time so a bar change can
be checked against the tempo.

```sh
cargo run --release --features mixer --example budget -- /tmp/budget
```

The voice budget as a measurement: N one-shots against a cap, with a scripted output that ends each sample. It writes
the live count and the refusals per frame. Before the `mixer` backend retired an ended voice, a cap of 32 held only
the first 32 footsteps and quietly stole or refused every one after that.

## What you can do

- **Name a sound once and let the crate choose the recording** — a `Cue` has variants, a gain, a pitch range, a
  repeat rule (`AvoidRepeat { avoid: 1 }` never plays the same variant twice in a row), a polyphony limit, a
  priority and a spatial model. Every play answers with `Played` or `Refused`, never silence:
  [cues](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/cues.md).
- **Mix in buses, not in sounds** — `Music`, `Effects`, `Ambience` and `Dialogue` out of the box, each with its own
  level, mute and voice cap; `Dialogue` ducks `Music` by 8 dB and `Effects` by 4 dB while a line plays, holding a
  quarter second and recovering over half a second. `SirenMix` is this frame's mix, one number per bus, for your
  own meter or an editor panel: [buses](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/buses.md).
- **The same seed replays the same sounds** — every variant pick and detune comes from one stream seeded by
  `SirenSettings::seed`, so a capture, a replay and an A/B of an audio change are reproducible; two seeds are two
  streams from the first play.
- **Spend voices on what a player is listening to** — a full budget steals the lowest priority, then the oldest,
  and a cue at `UNSTEALABLE_PRIORITY` (music, a stinger, a line) is never taken; a request that cannot be made room
  in is `Refused { VoiceLimit }` with nothing spawned, so a game that says 32 voices gets 32.
- **Change the music on the bar** — `MusicScore` is the layers a track is made of and its tempo; `MusicLayers`
  moves their gains on the next beat or bar (`next_boundary` from tempo, no beat map needed), and a layer at
  −80 dB stays in the mix so every layer is phase-aligned. A `Stinger` is the exception: it arrives at once and
  ducks the music it plays over: [music](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/music.md).
- **Place a sound in the world** — `Play::cue("bang").at(position)` attenuates by the cue's `Attenuation`
  (`Inverse` at 4 m and rolloff 1 by default), refusing with `SpatiallyMute` when there is no `SpatialListener`
  rather than guessing: [spatial](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/spatial.md).
- **Describe a sound and find it, behind `semantic`** — a folder of audio becomes an index keyed by content, a
  description becomes a ranked answer through CLAP's own tokeniser and spectrogram, and every result carries the score
  that explains it. The spectrogram is **CLAP's, to 0.0545 dB**, and the audio tower's embedding matches
  `laion/clap-htsat-fused` at **cosine 0.999999995** — both measured against the reference, not assumed. **Re-indexing
  embeds only what changed**: adding one file to a hundred costs one embedding, not a hundred, and a renamed file costs
  none. **Every asset carries its licence and source**, read from a pack's own sidecar during the build and reported on
  every result, with an explicit unknown that never reads as permission. **Each asset carries its measured loudness
  too**, taken from the decode the embedder already has, with a short clip's absent loudness stored as absence rather
  than as a sentinel number. **Each asset also carries its spectral shape** — centroid, rolloff at 95 % and noise
  floor — so two sounds from the same room can be found by brightness and cleanliness, and a **loudness or noise-floor
  filter removes a mismatch while an unmeasured asset always survives**. **Tempo is estimated per track with a ±2 BPM
  floor that is a test over 80 known tempi**, and a non-rhythmic asset reports *no tempo* rather than a confident wrong
  one. **Key and mode are detected too** — Krumhansl-Schmuckler, 100 % exact on a 24-key synthetic corpus, with an
  atonal asset reporting *no key* rather than a confident wrong one; a build produces both without a caller supplying
  them. The whole index is **binary, 10.28 MB for 5 000 clips** against 31.88 MB as text, and decoding one runs at
  **~450 clips/s** — a 5 000-clip library costs seconds, not minutes. Off by default, and a crate that does not enable
  it sees no change: `cargo tree` reads the same 413 lines either way:
  [semantic](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic.md).
- **Ship the sound as data, and edit it in the Cauldron** — with `serde`, the cue table, the mix table and the score
  are RON files; `bevy_grimoire`'s audio panel edits this crate's own types in
  `<project>/family/bevy_siren.ron`, never a second schema:
  [cauldron](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/cauldron.md).

## Bevy compatibility

| `bevy_siren` | Bevy | MSRV |
|---|---|---|
| 0.1 | 0.20 | 1.97.1 |

## Learn more

- The design rules and their sources: [docs/research/siren.md](https://github.com/bevy-coven/bevy_siren/blob/main/docs/research/siren.md).
- Every guide page: [docs/guide](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/README.md).
- What each iteration did and what is next: [docs/loop/log.md](https://github.com/bevy-coven/bevy_siren/blob/main/docs/loop/log.md).

## A Coven crate

Part of the [Bevy Coven](https://github.com/bevy-coven): small crates that each do one thing for a Bevy game and
say what they do not own. `bevy_siren` does not open files, does not own the output device, and does not own the
game's states: Bevy's `AudioPlugin` opens the device, the game says when to pause, and `SirenSounds` maps the keys
a project names.

## Who and licence

Written by [C. Thomas Brittain](https://github.com/Ladvien) and the agents of the Coven.

MIT OR Apache-2.0, at your option.
