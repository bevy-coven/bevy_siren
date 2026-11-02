# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-02

The first release: sound for Bevy 0.20 as data, mixed in buses.

### Added

- **A cue table as data.** `CueTable` of `Cue`s, each with variants, a bus, a gain, a pitch range, a repeat rule, a
  polyphony limit, a priority and an attenuation. Every play answers exactly one `Played` or `Refused`, and a refusal
  names what is missing (an unknown cue, no variants, a missing sound key, an unknown bus, a full budget, no listener).
- **Buses that duck.** `SirenSettings` with `Music`, `Effects`, `Ambience` and `Dialogue` out of the box, per-bus level,
  mute and voice cap, and `Duck` rules that lower one bus under another for a hold and a fade. `SirenMix` is this
  frame's mix — one number per bus, ducks and master included — for a panel or a game's own meter.
- **A voice budget that refuses politely.** Per-cue, per-bus and table-wide caps, with stealing by lowest priority then
  oldest, an `UNSTEALABLE_PRIORITY` above which nothing is taken, and a `Refused { VoiceLimit }` — nothing spawned —
  when the budget cannot be made room in.
- **Layered music on the bar.** `MusicScore` of `MusicLayer`s and a tempo; `MusicLayers` moves their gains with
  `When::{Now, NextBeat, NextBar}`, quantised by `model::next_boundary` from tempo alone. A layer at `SILENT_DB` stays
  in the mix so every layer is phase-aligned; a `Stinger` plays at once and ducks the music.
- **Spatial attenuation.** `Play::at(position)` with `Attenuation::{None, Inverse, Linear}`, Bevy's `SpatialListener` as
  the ears, and the backend panner's own distance law divided back out so a cue's curve is the one a project hears.
- **A log of what played and why.** `SirenLog` keeps the last 256 decisions — played, refused, stopped, ducked — each
  with the real time it happened.
- **Two backends.** Bevy's `AudioPlayer` by default, or the crate's own summing graph on `rodio` behind the `mixer`
  feature, where a bus's gain is one number per bus per frame.
- **The tables as RON**, behind the `serde` feature, with `from_ron`/`to_ron` on `CueTable`, `SirenSettings` and
  `MusicScore`, and a `problems()` on each that returns sentences a person reads.
- `examples/mixer` — the audio panel with its capture knobs, and `examples/music` — a scripted calm → combat → calm
  that checks every transition landed on a bar.

[Unreleased]: https://github.com/bevy-coven/bevy_siren/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/bevy-coven/bevy_siren/releases/tag/v0.1.0
