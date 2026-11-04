# bevy_siren — the design rules and where they come from

`bevy_siren` is small on purpose: a cue table, buses, ducks, a voice budget, layers and a log. None of those is an
invented feature; each is what a shipped game's audio system does, and each rule below says which source it rests on
and whether it is **measured** here or **house style** (a decision the Coven made, consistent with its other crates).

The note doubles as the reading list for anyone changing the crate: `## Sources` is what was read, `## The rules` is
what was taken from it, and `## Deviations` is where this crate chose differently and why.

## Sources

1. **Jason Gregory, *Game Engine Architecture*, 3rd ed.** — §14.5.8 *Audio and the Game Engine* and §14.6.3
   *Multichannel Mixing*. Read in home-still (`distill_search` "game audio mixing buses categories snapshots
   ducking"). What it establishes: sounds are grouped into **categories/buses**, not addressed individually; a mix is a
   set of **bus levels plus a master**; **ducking** is a group lowering another group for a while; **mix snapshots**
   are saved bus states; a mixer enforces **per-group instance limits**; a **virtual voice** is a sound the game
   believes is playing but that has no hardware voice; **voice stealing** takes by priority then age.
2. **Simon Franco, *Creating Dynamic Soundscapes Using an Artificial Sound Designer*, Game AI Pro ch. 46.** Read in
   home-still. What it establishes: a play *request* carries a **type** and a **subject** plus the facts the designer
   needs (a footstep carries the material it stepped on), and the system decides which sample and how loud.
3. **Khan et al. 2023, *Adaptive Background Music for a Fighting Game*,** arXiv:2303.15734. What it establishes:
   adaptive music is **per-instrument gain keyed to game state** — vertical layering is volume, not new tracks — with
   the same tempo and beat throughout.
4. **Kaushik 2025, *Procedural Music Generation in Video Games*,** 10.36948/ijfmr.2025.v07i02.39384. What it
   establishes: shipped games **pre-generate variation tables and choose at runtime**, rather than synthesising on the
   fly — which is why a cue's variants are data and the picker is a choice.
5. **Böttcher, *Procedural Audio* (2008).** What it establishes: the hard part of game audio is **managing thousands
   of assets and the workflow around them**, and repetition is fought with **alternative recordings and
   randomisation** — not with a cleverer synthesiser.
6. **Bevy 0.20's own audio path**, read in the vendored registry source (`bevy_audio-0.20.0/src/{audio.rs, sinks.rs,
   volume.rs, audio_output.rs}`): `AudioPlayer`, `PlaybackSettings`, `AudioSink`/`AudioSinkPlayback`, `SpatialListener`,
   `GlobalVolume`, `Volume::{Linear, Decibels}`, `AudioOutput` opening the default device and warning
   `No audio device found.` when there is none. What it establishes: the platform the crate builds on, and what a
   headless test can and cannot observe.
7. **`rodio` 0.22.2**, the version `bevy_audio` resolves: `Mixer`, `MixerDeviceSink::open_default_sink`,
   `Player::connect_new(&Mixer)`, `Source::{amplify, repeat_infinite, speed}`, and `source::Spatial`'s per-ear
   distance law. What it establishes: the graph the `mixer` feature builds, and the panner's arithmetic the crate
   divides back out.

## The rules

### R1 — A cue is a request, not a file. (measured: 2, 4, 5)

Gameplay names a **cue** (`"footstep.grass"`), and the crate chooses the recording, the pitch and the gain. Variants
are a table with a **no-repeat window** (`Repeat::AvoidRepeat { avoid }`) so one sample is never heard twice in a row,
which is Böttcher's answer to repetition and Kaushik's reason variation is pre-generated data.
*Where:* `src/cue.rs` (`Cue`, `SoundSlot`, `PitchRange`, `Repeat`), `src/model.rs` (`VariantPicker`).
*Proved by:* `tests/model.rs::the_picker_never_repeats_within_its_window`,
`tests/play.rs::the_same_cue_twice_never_repeats_a_variant`.

### R2 — Categories, not signals. (measured: 1)

The mix is **buses plus a master**, and a game sets a bus's level, not a cue's. `Music`, `Effects`, `Ambience`,
`Dialogue` are the four every project starts with.
*Where:* `src/mix.rs` (`SirenSettings`, `BusLevel`, `DEFAULT_BUSES`), `src/lib.rs` (`SirenMix`).
*Proved by:* `tests/mix.rs::a_muted_bus_is_silent_and_keeps_its_level`.

### R3 — A duck is an envelope, not a volume write. (measured: 1)

A voice starting on a **source** bus lowers a **target** bus by `by_db` for `hold_s`, then recovers over `fade_s`. The
envelope is evaluated every frame from **real time**, so a fade keeps running while the game is paused, and two ducks do
not stack — the deeper wins. The default pair is dialogue lowering music by 8 dB and effects by 4 dB, holding 0.25 s.
*Where:* `src/mix.rs` (`Duck`, `DuckClock`, `active_duck_db` in `src/model.rs`).
*Proved by:* `tests/model.rs::a_duck_holds_then_recovers`,
`tests/mix.rs::dialogue_ducks_the_music_and_recovers`.

### R4 — The voice budget refuses rather than grows. (measured: 1)

Per-cue polyphony, per-bus cap and a table-wide budget are three hard caps. A full budget steals the **lowest
priority, then the oldest**, never a voice at `UNSTEALABLE_PRIORITY` (music, stingers, dialogue); a request that cannot
be made room in is `Refused { VoiceLimit }` with **nothing spawned**.
*Where:* `src/model.rs` (`Candidate`, `steal`), `src/play.rs` (`room_for`).
*Proved by:* `tests/model.rs::steal_takes_low_priority_then_the_oldest`,
`tests/mix.rs::{a_full_bus_steals_the_oldest_and_then_refuses, the_table_budget_refuses_rather_than_grow,
a_cue_respects_its_own_polyphony}`.

### R5 — Layering is gain, and it lands on the bar. (measured: 3)

A score is layers of the same performance; a layer at `SILENT_DB` stays in the mix so every layer is phase-aligned.
A change is quantised to the next beat or bar **from tempo alone** (`next_boundary`), so no beat map is authored; with
`tempo_bpm: 0` every change is immediate. A **stinger** is the exception: it plays at once, on the music bus, and ducks
it.
*Where:* `src/music.rs` (`MusicScore`, `MusicLayer`, `MusicLayers`, `When`, `Stinger`), `src/model.rs`
(`next_boundary`).
*Proved by:* `tests/model.rs::the_bar_is_two_seconds_at_120_bpm`, `tests/music.rs::a_change_waits_for_the_bar`,
`tests/music.rs::a_silent_layer_stays_in_the_mix`, `tests/music.rs::a_stinger_ducks_the_music`.

### R6 — Silence is one number, and everything a play decides is readable. (house style; consistent with 1)

`SILENT_DB = −80` is where a gain becomes exactly `0.0` on the way in and out, so "is this bus audible" is one
comparison. A chain of gains **adds in dB** and any silent stage silences the whole (a cue at −80 dB is not brought
back by a bus at 0 dB), while several sounds **sum in linear** (two 0 dB sounds are one at +6.02 dB). And a play is
never silent: `Played` or `Refused`, both with the reason — **and never both**. `Played` is a promise that a voice exists, a handle
a game may keep and hang its own data on, so a play whose bytes the backend that will decode them cannot read is refused *at the
play* on either backend, before a voice is spawned (iterations 117 and 118). The postcondition is a contract and failing it is a bug
(Hunt and Thomas, *The Pragmatic Programmer*, tip 37); a handle goes stale, so a system that hands one out must not hand out one it
has already destroyed (Gregory, *Game Engine Architecture*, §16.5.3); and an invariant checked across scenarios is a partial test
oracle (Barr et al., *The Oracle Problem in Software Testing*, IEEE TSE 2015, §4.1). `SirenMix` is one number per bus, so a panel can
draw it.
*Where:* `src/model.rs` (`SILENT_DB`, `db_to_gain`, `gain_to_db`, `sum_db`, `chain_db`), `src/log.rs`, `src/sounds.rs` (`cannot_play`),
`src/output.rs` (`AudioOutput::decode_error`).
*Proved by:* `tests/model.rs::{db_to_gain_is_the_amplitude_ratio, silence_is_exactly_zero_and_never_negative_infinity,
a_sum_of_two_equal_sounds_is_plus_six_db, a_chain_adds_and_a_silent_stage_silences_it}`,
`tests/play.rs::an_unknown_cue_is_refused_and_spawns_nothing`,
`tests/one_answer.rs::{bytes_the_crates_mixer_refuses_are_refused_at_the_play_and_nothing_is_announced,
every_play_is_answered_once_and_a_voice_that_is_gone_is_explained}`.

## How shipping games do it

Quoted, not paraphrased:

- Gregory on grouping: *"sounds are grouped into categories or buses, and the mix is controlled by adjusting the
  level of each bus rather than by adjusting each sound individually."* That is R2, and it is why a duck is a
  crossfade between two bus levels rather than a script that walks every sound.
- Gregory on voice limits: *"the number of simultaneous sounds in a group is limited, and when the limit is reached,
  the sound with the lowest priority (or the oldest) is stolen."* That is R4, including the order.
- Franco on requests: *"the game emits a play request that says what kind of sound it wants and which object it
  belongs to, and the audio system decides which sample to play."* That is R1, and it is why a footstep carries its
  material as part of the cue's name rather than as a branch in gameplay.
- Khan et al. on layering: adaptive music is *"a vertical layering technique, where the stems of a single piece are
  mixed in and out according to the game state"* — the same performance, not a second track. That is R5.
- Böttcher on repetition: *"the main problem of procedural audio is not the synthesis but the management of thousands
  of assets"*, met with *"alternative recordings and randomisation"*. That is R1's variant table and its no-repeat
  window.

## Model adopted

```
Play { cue, target, position, gain, pitch }
  │
  ├─ cue table lookup ─── unknown ────────────────────────► Refused::UnknownCue
  ├─ variants ─────────── none ───────────────────────────► Refused::NoSounds
  ├─ picker (no-repeat window) + pitch jitter
  ├─ sounds key ──────── no handle ───────────────────────► Refused::MissingSound
  ├─ bus ─────────────── not in the mix table ────────────► Refused::UnknownBus
  ├─ room_for (cue cap → bus cap → table budget; steal) ──► Refused::VoiceLimit
  ├─ spatial? no listener ────────────────────────────────► Refused::SpatiallyMute
  └─ spawn Voice (+ AudioPlayer | MixerVoice) ────────────► Played { entity, sound, bus, gain, pitch }

mix (real time): bus target = master + bus gain + strongest duck
                 each bus approaches its target at MIX_TAU_S
                 each voice's sink = voice gain + bus, written only when it moved
```

## Parameters

| Name | Value | Why |
|---|---|---|
| `SILENT_DB` | −80 dB | 1/10 000 of full scale: below it a gain is silence, and one number decides what silence is |
| `DEFAULT_FADE_S` | 0.12 s | a stolen or stopped voice fades without a click, short enough to read as immediate |
| `MIX_TAU_S` | 0.05 s | one time constant of a bus move: ~150 ms to settle, inaudible as a step |
| `max_voices` | 32 | more than a busy scene's close effects, fewer than a mixer's worth of sample cost |
| bus `max_voices` | 0 (the table's) | a bus cannot raise the budget by itself |
| default duck | −8 dB music, −4 dB effects, hold 0.25 s, fade 0.6 / 0.4 s | Gregory §14.6.3's dialogue duck, the one pair every game wants |
| `UNSTEALABLE_PRIORITY` | 180 | music, stingers and dialogue are what a player listens *to* |
| default attenuation | `Inverse { near_m: 4, rolloff: 1 }` | a point source that is full at the listener's feet and halves at 8 m |
| default pitch range | ±1 semitone | four plays of one footstep are four sounds and no one notices the pitch |
| `SirenLog::CAPACITY` | 256 | a panel shows eight; a test reads one; a person asking "why" reads the rest |
| seed | `0x5EED_51AE_0000_0001` | the same seed replays the same sounds, so a capture and a test are reproducible |

## Deviations

- **A per-cue cap and a bus cap steal rather than queue.** Gregory describes stealing for a group; this crate applies
  it at the cue as well, because a footstep that queues behind two others is a footstep that plays late. That is a
  house-style reading of the same rule, and `tests/mix.rs::a_cue_respects_its_own_polyphony` pins it.
- **The mixer does not pan.** A rodio source cannot be re-positioned once inside a `Player`, so the `mixer` feature
  applies spatial attenuation at play time and Bevy's two-ear path remains the default for moving emitters. Stated in
  `src/mixer/mod.rs` and `docs/guide/spatial.md`; a game that needs panning keeps the default.
- **The backend panner's distance law is divided out.** `rodio::source::Spatial` multiplies each ear by
  `min(1, 1/d²)` as well as panning, so a cue's own `Attenuation` would be applied twice. `model::backend_spatial_gain`
  mirrors that formula and the mix divides it back out, which is a deviation from "the panner decides loudness" in
  favour of "the cue table decides loudness".
- **The mix runs on real time, not the game clock.** A duck must recover behind a paused menu, and a fade must not
  freeze at 40%. House style, consistent with `bevy_gooey`'s `Meter` reading `Time<Real>`.

## Validation targets

- A cue asked twice never repeats a variant (R1).
- Dialogue at −8 dB on music recovers to 0 dB within `hold + fade` (R3).
- A 3-voice request against a bus capped at 2 steals the oldest; a request against an all-unstealable bus is refused and
  spawns nothing (R4).
- At 120 bpm and four beats a bar, a `NextBar` change from 0.3 s lands at 2.0 s, and no gain moves before it (R5).
- Every `Play` produces exactly one `Played` or one `Refused`, never both and never neither (R6).

## Pending index

Nothing in this note is unindexed: Gregory's chapter, Franco's chapter and the two papers were read through home-still
and are in its corpus. The Bevy and rodio facts are vendored source, not papers, and are cited by file.
