# bevy_siren

> ⚠️ **Vibe Coded.** Written by AI agents working from a human's direction, covered by tests rather than a line-by-line human review. Read the code before you rely on it for something you care about.

Sound for Bevy 0.20 that decides *what* plays and *how loud*: cues with variants, buses that duck, a voice budget that refuses politely, music that changes on the bar, and a log of what played and why.

[![crates.io](https://img.shields.io/crates/v/bevy_siren.svg)](https://crates.io/crates/bevy_siren)
[![docs.rs](https://docs.rs/bevy_siren/badge.svg)](https://docs.rs/bevy_siren)
[![CI](https://github.com/bevy-coven/bevy_siren/actions/workflows/ci.yml/badge.svg)](https://github.com/bevy-coven/bevy_siren/actions/workflows/ci.yml)
[![Bevy](https://img.shields.io/badge/bevy-0.20-blue.svg)](https://bevyengine.org)
[![licence](https://img.shields.io/badge/licence-MIT%20OR%20Apache--2.0-blue.svg)](#who-and-licence)

**Status:** usable · Bevy 0.20 · MSRV 1.97.1

**Why:** Bevy gives you an `AudioPlayer` and a mixer. What every game still rebuilds on top is the part that decides *what* to play and *how loud* — a footstep that does not machine-gun, dialogue that steps the music back, a firefight that does not spawn four hundred sinks, a track whose combat layer comes in without cutting the tune, and a line that says why the music went quiet. `bevy_siren` makes that one crate's worth of it shared.

## Quick start

```sh
cargo add bevy_siren
```

```rust,no_run
use bevy::prelude::*;
use bevy_siren::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SirenPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (walk, report))
        .run();
}

fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
    // A cue with two variants: the picker never plays the same one twice in a row.
    // A sound key is a file's name under `assets/audio/`, extension included.
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01.ogg"), SoundSlot::new("step_02.ogg")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

// A footstep every half second.
fn walk(mut play: MessageWriter<Play>, time: Res<Time>, mut elapsed: Local<f32>) {
    *elapsed += time.delta_secs();
    if *elapsed >= 0.5 {
        *elapsed -= 0.5;
        play.write(Play::cue("footstep.grass"));
    }
}

// What the crate decided, once each, in its own words: which recording played, or why nothing did.
fn report(log: Res<SirenLog>, mut seen: Local<u64>) {
    for entry in log.since(*seen) {
        info!("{}", entry.line());
    }
    *seen = log.pushed();
}
```

A sound key is a file's name under `assets/audio/`, extension included. Put two short recordings there as `step_01.ogg` and
`step_02.ogg`, or run `cargo run --example quick_start` in a clone, which plays the two this repository ships. The console
is the crate's own log, so a footstep reads ``0.50s played `footstep.grass` (step_02.ogg) on Effects at 0.00 dB``, a
misspelt cue reads ``refused `footstep.grss`: no cue of that name`` **without any code of yours printing it**, and a key
with no file reads ``refused `footstep.grass`: the file for sound `step_03.ogg` failed to load: Path not found:
…/assets/audio/step_03.ogg``.

This block is `examples/quick_start.rs`, the file `tests/quick_start.rs` **runs** (the real asset server, Bevy's own audio
loader, the two files, the console read back), and the crate-root doc and this README are held to it:
`cargo test --features readme-example --test all readme::`.

From git, for changes not released yet:

```toml
[dependencies]
bevy_siren = { git = "https://github.com/bevy-coven/bevy_siren" }
```

Features: `serde` (the cue table, the mix table and the score as RON) and `mixer` (the crate's own summing graph, one
number per bus per frame). Both answer `Played`, `Refused` and `Voice` identically; only the sample path differs.

## Try it

**▶ [Listen to the three A/B pairs — 26 seconds](https://cdn.jsdelivr.net/gh/bevy-coven/bevy_siren@b47053755020a5a58e51f90f1ca4b54c96562a27/docs/media/demo/tour.mp4)**
(577 KB MP4, served as `video/mp4` so a browser plays it in place), or `cargo run --example demo` for the crate working
live in a window.

[![A frame of the tour: a large B beside "vignette 1: variant selection", the sentence "A is one recording played ten times at a walk's cadence — a machine gun. B is the same cue with three recordings and a no-repeat window", two waveforms marked A and B, and their levels at −43.4 and −42.8 dBFS](https://cdn.jsdelivr.net/gh/bevy-coven/bevy_siren@b47053755020a5a58e51f90f1ca4b54c96562a27/docs/media/demo/tour-poster.png)](https://cdn.jsdelivr.net/gh/bevy-coven/bevy_siren@b47053755020a5a58e51f90f1ca4b54c96562a27/docs/media/demo/tour.mp4)

*The tour, mid-`B`. Click to play.*

**Each pair changes one thing, and the two are matched to within 0.5 LU**, so what you hear is the feature and not a
level difference. That is the whole design:

| # | what it demonstrates | A — without the crate | B — with it |
|---|---|---|---|
| 1 | **variant selection** | one recording played ten times at a walk's cadence: a machine gun | the same cue with three recordings and a no-repeat window: a walk |
| 2 | **the voice budget** | forty footsteps at once, forty voices: the sum turns to sludge | the same forty against a cap of four: older voices are taken back, the mix stays legible |
| 3 | **distance** | the same level at 4 m and at 16 m | the distance law: the far one arrives 12 dB lower, at 4 m and 16 m |

The picture shows each half's waveform and its measured level, so a difference you can hear is also one you can see.
`docs/media/demo/tour.txt` is the run's own report: every half's peak, RMS, integrated loudness, the gain applied to match
it, and **the crate's log lines**.

> **Two pairs were cut, and that is worth saying.** *Ducking* and *the layer change on the bar* both need a bed that keeps
> playing while something happens over it. On this crate a sustained bed is either a `Music` stem — which the score adopts
> and drives, so its level is the score's to set — or a cue the table's budget refused in the harness. The duck rule
> **was** measured working (`dialogue.greeting ducked Ambience by -8.0 dB`), but with nothing under it to duck the A and B
> halves correlated at **0.997**: a pair that demonstrates nothing. They are the next iteration's item rather than an A/B
> here that is not one.

**The demo that came before this one is gone on purpose.** It put nine features in one continuous stream with a music pad
and a noise bed under them, so its quietest half-second after the fifth second was **−28 dBFS** and it was heard — rightly —
as a hum. The music files were three pure sines (spectral flatness 0.0005). The sound set now uses **plucked phrases that
decay to silence between notes** (31–34 % of each music file is below −60 dBFS), and the demo is A/B pairs rather than a
tour.

**No music-bed demo of the crate's own.** `cargo run --example mixer` is the audio panel: every bus with its level, its
duck and its live voice count, the cue table with the crate's verdict on it, and the log of what played and why.
`SIREN_KEYS=steal,layer,duck` stages a run so a capture shows the three things this crate exists for; `SIREN_FAULTS=1`
adds two deliberate mistakes so the `Problems` block does something.

## What you can do

- **Name a sound once and let the crate choose the recording** — a `Cue` has variants, a gain, a pitch range, a repeat
  rule (`AvoidRepeat { avoid: 1 }` never plays the same variant twice in a row), a polyphony limit, a priority and a
  spatial model. Every play answers with `Played` or `Refused`, never silence: [cues](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/cues.md).
- **Mix in buses, not in sounds** — `Music`, `Effects`, `Ambience` and `Dialogue` out of the box, each with its own level,
  mute and voice cap; `Dialogue` ducks `Music` by 8 dB and `Effects` by 4 dB while a line plays, holding a quarter second
  and recovering over half a second. `SirenMix` is this frame's mix, one number per bus, for your own meter or a panel:
  [buses](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/buses.md).
- **Spend voices on what a player is listening to** — a full budget steals the lowest priority, then the oldest, and a cue
  at `UNSTEALABLE_PRIORITY` (music, a stinger, a line) is never taken; a request that cannot be made room in is
  `Refused { VoiceLimit }` with nothing spawned, so a game that says 32 voices gets 32.
- **The same seed replays the same sounds** — every variant pick and detune comes from one stream seeded by
  `SirenSettings::seed`, so a capture, a replay and an A/B of an audio change are reproducible. **Any number is a good
  seed** (it is mixed before use), and **a play that cannot decide takes nothing from the stream**: a cue with an exact
  pitch and a pick whose only candidate is left move no other cue's choices
  ([the seed](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/cues.md#the-seed)).
- **Change the music on the bar** — `MusicScore` is the layers a track is made of and its tempo; `MusicLayers` moves their
  gains on the next beat or bar (`next_boundary` from tempo, no beat map needed), and a layer at −80 dB stays in the mix so
  every layer is phase-aligned. A `Stinger` is the exception: it arrives at once and ducks the music it plays over:
  [music](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/music.md).
- **Place a sound in the world** — `Play::cue("bang").at(position)` attenuates by the cue's `Attenuation` (`Inverse` at 4 m
  and rolloff 1 by default), refusing with `SpatiallyMute` when there is no `SpatialListener` rather than guessing.
  `.on(entity)` hangs the sound from an entity — it ends when the entity does, and a placed one follows it — and
  `.at(position)` stays a world position whatever the entity is:
  [spatial](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/spatial.md).
- **Ship the sound as data, and edit it in the Cauldron** — with `serde`, the cue table, the mix table and the score are
  RON files; `bevy_grimoire`'s audio panel edits this crate's own types in `<project>/family/bevy_siren.ron`, never a
  second schema: [cauldron](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/cauldron.md).
- **Describe a sound and find it, behind `semantic`** — a folder of audio becomes a content-keyed index, a description
  becomes a ranked answer through CLAP's own tokeniser and spectrogram, and every result carries the score that explains
  it and the asset's licence. Measured against two labelled libraries, honestly, including where it is weak:
  [semantic](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic.md),
  [the golden set](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/golden-set.md).
  **Try it in one command**: `cargo run --release --features semantic --example similarity_demo -- <folder>` asks six ordinary
  descriptions and marks whether each found its class — **2 of 6 do**, and the four that miss return engine drones, because
  one class is near everything (the golden set: **22 of 47** descriptions put an engine first). It then shows the fix that is *not* shipped:
  normalising by each file's mean similarity to the other descriptions lifts hit@1 from **9 of 48 to 14 of 48**, the
  direction measured at **31 % → 44.8 %** — [what a description finds, and why it often does not](https://github.com/bevy-coven/bevy_siren/blob/main/docs/media/similarity.png).

## Part of the Coven

`bevy_siren` is one of the [Bevy Coven](https://github.com/bevy-coven)'s small crates that each do one thing for a Bevy
game and say what they do not own. Using it beside the editor is [the book](https://bevy-coven.github.io/)'s audio page.

## Bevy compatibility

| `bevy_siren` | Bevy | MSRV |
|---|---|---|
| 0.1 | 0.20 | 1.97.1 |

## Learn more

- [docs.rs/bevy_siren](https://docs.rs/bevy_siren) — the API, every type documented.
- [The guide](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/README.md) — one page per thing a game does
  with sound: cues, buses, music, spatial, the Cauldron, the semantic layer.
- [The design rules and their sources](https://github.com/bevy-coven/bevy_siren/blob/main/docs/research/siren.md).
- [The examples](https://github.com/bevy-coven/bevy_siren/tree/main/examples) — `demo`, `quick_start`, `mixer`, `music`,
  `budget`, `perfcheck`, and the semantic tools.
- [The CHANGELOG](https://github.com/bevy-coven/bevy_siren/blob/main/CHANGELOG.md), and
  [what each iteration did](https://github.com/bevy-coven/bevy_siren/blob/main/docs/loop/log.md).

## Who and licence

Written by [C. Thomas Brittain](https://github.com/Ladvien) and the agents of the Coven. Issues and requests are at
[bevy-coven/bevy_siren/issues](https://github.com/bevy-coven/bevy_siren/issues); contributing is the org's
[`CONTRIBUTING.md`](https://github.com/bevy-coven/.github/blob/main/CONTRIBUTING.md).

MIT OR Apache-2.0, at your option.
