# Changelog

All notable changes to this project. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and the project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **`UNSTEALABLE_PRIORITY`'s rule is now stated directly in the suite.** It was proved through
  `the_table_budget_refuses_rather_than_grow`, whose second request is the *same* unstealable cue — a path that takes a
  paragraph to explain. Two tests now ask it the way the rule is written: a priority-0 request against a priority-200
  voice is refused with nothing stolen, and a stealable voice still plays beside one so neither can pass vacuously. No
  `src/` file changed; the mutation check (`steal` ignoring `unstealable`) reddens both new tests **and** the old one.

### Fixed

- **A `StopVoice` for a voice that has already ended no longer panics.** `stop_requests` took the entity straight from
  the message and issued a command on it; `retire_voices` despawns a one-shot the frame it ends, and `Played::entity` is
  documented as the handle a game keeps for `StopVoice` — so holding it one frame too long hit Bevy 0.20's default error
  handler, which **panics** (`Entity despawned … its index now has generation 1`). The request is now resolved against
  the live voice query first: there is nothing left to stop, so nothing happens and nothing is logged. The same guard
  makes a second `StopVoice` for a voice already fading a no-op, so a fade is begun once.
- **An empty `buses` list mixed every voice at −80 dB.** `answer`, the editor's panel and `SirenSettings::bus_names()`
  all treat a table listing no buses as the crate's four, but `update_mix` iterated `settings.buses` directly — so with
  `buses: []` a cue on `Effects` was **accepted and then mixed at −80 dB**, with no refusal and no log line. The mix now
  iterates the same resolved list `bus_names()` returns.
- **`SirenMix` no longer keeps a bus that left the table.** `db` and `ducked` were written by insertion and never pruned,
  so a project that swapped `SirenSettings` kept every bus from the previous table at its last level: `bus_db` answered
  for a bus that did not exist, `deepest_duck()` named one forever, and a removed bus's duck was frozen at its deepest.
  Both maps are now pruned to this frame's buses before the bus loop, which leaves the previous value readable for a bus
  that stayed — so a fader still eases rather than steps.
- **A stinger that cannot play no longer ducks the music.** `play_stingers` wrote the `Play` and armed its duck in the
  same breath, but `Play` is answered later in the frame — so a refused stinger left the `Music` bus down for a sound
  nobody heard, and `SirenLog` stated the duck **and** the refusal on adjacent lines. A new `arm_stinger_ducks` arms
  the duck when `Played` arrives, after `Play` and before `Mix`, so the dip still lands on the frame the stinger fires.
  Stinger refusals now write a `Refused` message as well as a log line, and a cue **not in the table** is refused as
  `UnknownCue` instead of inheriting `Music` through `unwrap_or`. A wrong-bus stinger has no `Refusal` variant that is
  true, so its sentence stays in the log: `docs/loop/needs-decision/stinger-refusal.md`.
- **A non-spatial cue given a position is no longer routed through the two-ear panner.** `spawn_voice` decided Bevy's
  `PlaybackSettings.spatial` — and whether to attach a `Transform` — from `request.position.is_some()` alone, so a cue
  authored `spatial: false` and played with `.at(..)` went through `rodio`'s panner, which applies `min(1, 1/d²)` as
  well as a pan: **−38.7 dB at 8 m**, while `Played.gain_db` and the log reported **0.00 dB**. One predicate now
  decides the attenuation, the panner and the `Transform` (`cue.spatial && position.is_some()`). **A project that
  relied on the accident loses a pan it was never promised, and its meter was wrong while it happened.** Refusing such
  a play instead is `docs/loop/needs-decision/non-spatial-position.md`.
- **The crate's own mixer reuses a voice slot instead of retaining every sound ever played.** `MixerBackend` pushed a
  `VoiceTrack` — a `rodio::Player` and its queues — per play and never removed one, so a game on the `mixer` backend
  held one player for every sound it had ever played: 400 one-shots held **400 slots** on a real device. `play` now
  reuses the first slot whose voice has ended, bounding the list by the peak *concurrent* count (6 for those 400), and
  the index it returns packs a slot **and a generation** (`model::pack_voice`), so a `MixerVoice` held by an entity the
  world has not finished despawning addresses nothing rather than the slot's next occupant. `MixerBackend::voices()`
  now reports slots, not sounds played. No public type or trait signature changed.
- **A sample the decoder refuses names the sound key, not the cue.** `start_pending` reported a decode failure as
  `MissingSound { sound: voice.cue }`, putting the cue's name (`footstep.grass`) in a field documented as the key a
  project maps to a file (`step_03`) — so a corrupt `step_03.wav` sent a person looking for a file that never existed.
  It now recovers the key from the same cue and variant `answer` chose. The variant's sentence covers both of its
  causes, *"no handle or undecodable bytes for sound `step_03`"*; a variant that distinguishes them is
  `docs/loop/needs-decision/undecodable-sound.md`. No public type or field changed.
- **A `VoiceLimit` refusal names the bus only when the bus refused.** `room_for` has three hard caps — the cue's
  `max_instances`, its bus's `max_voices`, the table's `max_voices` — and returned one uninformative `None` from all
  of them, so every refusal reported the cue's bus. A probe proved the table case blamed **`Ambience`** for a cue
  playing on `Effects`. It now returns the cap (private `Cap::{Cue, Bus, Table}`): `bus: Some(name)` only for the
  bus's own cap, `bus: None` otherwise, which is what the field's own doc says (*"the bus that had no room, when one
  did"*). **A host that matched `VoiceLimit { bus: Some(_) }` to mean "the limit refused at all" now sees it only for
  the bus case**; `VoiceLimit { .. }` still matches every one. A payload that also names the cap is
  `docs/loop/needs-decision/refusal-names-its-cap.md`.
- **`MusicLayers::fade_s` is the crossfade it says it is.** The field was accepted, stored in `Pending`, and read by
  nothing: `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms and
  nothing said so. `model::tau_for_fade` derives the time constant from `approach`'s own definition
  (`fade_s / ln(1000)`, an arrival within one part in a thousand), and each layer carries the fade it was last given.
  `fade_s(0)` keeps `MIX_TAU_S`, so the builder's default is unchanged.
- **A frame with no elapsed time holds a layer's gain.** `approach` returns its target for `dt <= 0`, which is right
  for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend the whole
  authored fade in one step. `move_layers` now holds the gain when `dt <= 0`.
- **`SirenSettings::seed` is now the stream the crate draws from.** `SirenChoices` was `init_resource`d from a crate
  constant and never rebuilt, so a project's `seed:` was read by nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. `play::seed_choices` replaces the stream (and every variant history) when the mix
  table's seed changes, so two seeds are two streams and one seed replays itself — `cues.yaml` case 17, which could
  not pass before. No field moved: the setting existed and now works.
- **A stinger's `duck_s` is a hold, not a fade.** `play_stingers` passed it as `DuckClock::manual`'s *fade* argument
  with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired: a −9 dB stinger read
  **−5.01 dB** at its midpoint and the music swelled back underneath it. It now holds for `duck_s` and releases over
  `DEFAULT_FADE_S` (0.12 s), reading **−8.99 dB** at the same point. A project-chosen release is
  `docs/loop/needs-decision/stinger-release.md`.
- **A voice that ended now frees its slot on the crate's own mixer.** With the `mixer` feature, a `Mode::Once`
  voice was never retired — the crate asked Bevy's `AudioSink`, which that backend does not create, and nothing ever
  set `Completes` — so it kept its place in the voice budget forever. After `max_voices` (32 by default) one-shots,
  every later play either stole an audible voice (with a full budget, silence was hidden behind a steal) or, for an
  unstealable cue, was refused `VoiceLimit` for the life of the process. `retire_voices` now asks
  `AudioOutput::finished`, the query the backend already implemented, and writes the same `ended` log line Bevy's
  path writes. Gregory §14.5.8.4: a stopped voice *"frees its resources for use by other voices"*.
- **A voice's own gain is written on both backends.** `update_mix` skipped a `MixerVoice` entirely, so a music layer
  moved by `MusicLayers` stayed at the gain it started at, and a stop fade began from silence and despawned on its
  first frame — a cut where the crate promises a fade. The mix now writes the voice's own gain into the mixer and
  its whole chain into a Bevy sink, records which in `Applied`, and one writer still owns both.

### Added

- `examples/budget` (`--features mixer`) — a headless run of N one-shots against a voice cap, with a scripted output
  that ends each sample, writing the live count and the refusals per frame as a CSV, a text summary and an SVG plot.
  It is the evidence harness for "an ended voice frees its slot", from `docs/loop/cues.yaml` cases 20 and 21.
- `tests/mixer.rs` — three tests behind the `mixer` feature: a one-shot's slot is freed (fails on the unfixed tree), a
  loop is not retired when its source ends, and a fading voice is not also reported ended.
- `examples/render` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  WAVs and a `render.txt`, so a change can be judged by spectrogram and loudness without an ear. No public API
  changes: it drives `bevy_siren::model` directly.
- `examples/perfcheck` — µs per frame and per voice at 8, 64 and 256 voices.
- `.omp/skills/build-loop` — the loop itself, as a project skill: one iteration from orient to log, with the four
  traps that cost an iteration each written down where the next run reads them.
- `docs/loop/cues.yaml` — the fixed cases every iteration runs, each with its observation and the source for its
  number; `docs/loop/runs/` and `docs/loop/needs-decision/` for a run's evidence and the questions the loop will not
  decide itself. `docs/loop-log.md` becomes `docs/loop/log.md`.
- `docs/loop/needs-decision/remove-completes.md` — whether the public `Completes` component, which cannot ever be
  true, is removed in the next minor version.

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
