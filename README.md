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

A sound key is a file's name under `assets/audio/`, extension included: put two short recordings there as
`step_01.ogg` and `step_02.ogg`, or, in a clone of this repository, `cargo run --example quick_start` plays the two it
ships. The console is the crate's own log, so a footstep reads ``0.50s played `footstep.grass` (step_02.ogg) on Effects
at 0.00 dB`` and a key with no file reads ``refused `footstep.grass`: the file for sound `step_03.ogg` failed to load:
Path not found: …/assets/audio/step_03.ogg`` — the cue, the key, why, and the path that was looked for.

This block is `examples/quick_start.rs`, the file `tests/quick_start.rs` **runs** (the real asset server, Bevy's own
audio loader, the two files, the console read back), and the crate-root doctest and this README are held to it:
`cargo test --features readme-example --test all readme::` checks the README.

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
  streams from the first play, and **any number is a good seed** — it is mixed before use, so `seed: 1` and `seed: 2` are as unrelated as two random 64-bit ones.
- **Spend voices on what a player is listening to** — a full budget steals the lowest priority, then the oldest,
  and a cue at `UNSTEALABLE_PRIORITY` (music, a stinger, a line) is never taken; a request that cannot be made room
  in is `Refused { VoiceLimit }` with nothing spawned, so a game that says 32 voices gets 32.
- **Change the music on the bar** — `MusicScore` is the layers a track is made of and its tempo; `MusicLayers`
  moves their gains on the next beat or bar (`next_boundary` from tempo, no beat map needed), and a layer at
  −80 dB stays in the mix so every layer is phase-aligned. A `Stinger` is the exception: it arrives at once and
  ducks the music it plays over: [music](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/music.md).
- **Place a sound in the world** — `Play::cue("bang").at(position)` attenuates by the cue's `Attenuation`
  (`Inverse` at 4 m and rolloff 1 by default), refusing with `SpatiallyMute` when there is no `SpatialListener`
  rather than guessing. `.on(entity)` hangs the sound from an entity — it ends when the entity does, and a placed one
  follows it — and `.at(position)` stays a world position whatever the entity is:
  [spatial](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/spatial.md).
- **Describe a sound and find it, behind `semantic`** — a folder of audio becomes an index keyed by content, a
  description becomes a ranked answer through CLAP's own tokeniser and spectrogram, and every result carries the score
  that explains it. The spectrogram is **CLAP's, to 0.0545 dB**, and the audio tower's embedding matches
  `laion/clap-htsat-fused` at **cosine 0.999999995** — both measured against the reference, not assumed. **Re-indexing
  embeds only what changed**: adding one file to a hundred costs one embedding, not a hundred, and a renamed file costs
  none. **Every asset carries its licence and source**, read from a pack's own sidecar during the build and reported on
  every result, with an explicit unknown that never reads as permission. **Each asset carries its measured loudness
  too**, taken from the decode the embedder already has, with a short clip's absent loudness stored as absence rather
  than as a sentinel number. **Each asset also carries its spectral shape** — centroid, rolloff at 95 % and noise
  floor — so two sounds from the same room can be found by brightness and cleanliness. A **filter removes a
  mismatch**: loudness and tempo exclude a pair, the noise floor and key only penalise one, and an asset that was never
  measured always survives. **Tempo is estimated per track with a ±2 BPM floor that is a test over 80 known tempi**,
  and a non-rhythmic asset reports *no tempo* rather than a confident wrong one. **Key and mode are detected too** — Krumhansl-Schmuckler, 100 % exact on a 24-key synthetic corpus, with an
  atonal asset reporting *no key* rather than a confident wrong one; a build produces both without a caller supplying
  them. The whole index is **binary, 10.28 MB for 5 000 clips** against 31.88 MB as text. Decoding a clip runs at
  **~750 clips/s**, but **inference is the bound**: the real audio tower, run end to end for the first time in iteration
  110, indexed 280 one-second clips at **16.6 clips/s on an M4 CPU** — 60 ms a clip, **about five minutes** for 5 000 — and 2 000
  five-second recordings at **9.5 clips/s, 105 ms a clip** (about nine minutes for 5 000, measured under load).
  `incremental::update` pays only for what changed; `examples/semantic` rebuilds whole. Off by default: a
  build that does not enable it is unchanged at **413 lines** of `cargo tree --edges normal`, and with it that is
  **497**. The pipeline and its rules are in
  [semantic](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic.md); every authored number — each
  weight, threshold and variant distance, with its default and what it does at its extremes — is in
  [semantic-config](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic-config.md).
  **The crate does not ship the model.** A project loads an ONNX export of `laion/clap-htsat-fused` (MIT) itself, and the
  index records which export made its vectors so a mismatch is refused rather than ranked; the export this crate measured
  is `transformers-5.22.1/torch-2.15.0/opset-17`, and `--model-version` (or `ModelStamp::clap`) names any other.
  **Try it on your own files**: `tools/export_clap_onnx.py` makes both graphs from the weights in your Hugging Face cache,
  and `examples/semantic` indexes a folder, takes a description, prints the best three with the reason for each and plays
  one — [the guide](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic.md#try-it-examplessemantic).
  **It is measured against two libraries**: on 2 000 ESC-50 recordings, fifty descriptions written by the maintainer reach
  **hit@1 90 %, hit@3 100 %** — and **72 % and 84 %** when the same sounds are described in a stranger's words — and the pipeline
  reproduces `transformers`' zero-shot accuracy on the same audio to four places (**87.8 %**). **On 303 game sounds from Kenney's CC0
  packs it is far weaker: hit@1 31 %, hit@3 52 % (38 % in a stranger's words)**, the right answers' median similarity is 0.19 against
  0.47, and one class (`engine`) is the first result for 22 of 47 descriptions — a hub. The first run overturned a documented claim —
  the right and wrong score bands do *not* overlap much on real recordings — and **the second showed that the first library's floor
  does not transfer**: a similarity floor that costs nothing on ESC-50 (0.35) returns nothing at all for 13 of 29 descriptions of game
  sounds, so the default stays `0.0` and the floor is [a decision](https://github.com/bevy-coven/bevy_siren/blob/main/docs/loop/needs-decision/default-floor.md).
  Two libraries, neither of them every library: [what it cannot say](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/golden-set.md#what-it-cannot-say).
  **An agent can drive all of it**: `siren-tools` is an MCP server over
  stdio — six tools, one error shape, and a bad call is a result a model can read, not a protocol error it never sees —
  built with `--features semantic` and nothing else, in [the guide](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic.md#the-tool-server-siren-tools).

  ![`examples/semantic` on a folder of fourteen WAV files, as a terminal transcript: the first run builds the index with the real CLAP audio tower in 1.1 s, ranks "a short electronic beep" with Sosumi first, and plays it through the Music bus; the second run reads the index it wrote, loads no audio graph, and ranks "a frog croaking" — a miss, with the reason printed under every result.](https://raw.githubusercontent.com/bevy-coven/bevy_siren/main/docs/media/semantic.png)

  *`examples/semantic`, 1325 × 1056. The second query is a miss worth reading: CLAP ranks the system sound named `Frog`
  eleventh of fourteen (cosine −0.02, below the floor) because it is a synthetic blip, not a frog.*
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
