//! **Sound for Bevy games**: a cue table as data, buses that duck, a voice budget, layered music on the bar, and
//! a log of what played and why.
//!
//! Bevy gives you an `AudioPlayer` and a mixer. What a game needs on top is the part every game rebuilds:
//! a way to say *"play the grass footstep"* and get a different recording each time, a bus structure with
//! ducking so a line of dialogue steps the music back, a voice budget that refuses politely instead of
//! dropping the newest sound, music layered by instrument so a fight can start without cutting the track,
//! and an answer to "why is the music quiet" that is not five hours with a debugger.
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_siren::*;
//!
//! fn main() {
//!     App::new().add_plugins((DefaultPlugins, SirenPlugin)).add_systems(Startup, setup).run();
//! }
//!
//! fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
//!     // A cue with two variants: the picker never plays the same one twice in a row.
//!     cue_table.insert(Cue {
//!         sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
//!         ..Cue::new("footstep.grass")
//!     });
//!     sounds.load_table(&assets, &cue_table, "audio");
//! }
//!
//! fn walk(mut play: MessageWriter<Play>) {
//!     play.write(Play::cue("footstep.grass"));
//! }
//!
//! fn hear(mut played: MessageReader<Played>, mut log: ResMut<SirenLog>) {
//!     for answer in played.read() {
//!         // Which recording the player heard, and why it was chosen.
//!         log.push(Entry::Played {
//!             at_s: 0.0,
//!             cue: answer.cue.clone(),
//!             sound: answer.sound.clone(),
//!             bus: answer.bus.clone(),
//!             gain_db: answer.gain_db,
//!         });
//!     }
//! }
//! ```
//!
//! # The pieces
//!
//! - [`CueTable`] — what a game can play, by name: variants, a bus, a gain, a pitch range, a repeat rule, a
//!   polyphony limit, a priority and a spatial model. Plain data, and with the `serde` feature a RON file.
//! - [`Play`] — one request, answered by exactly one [`Played`] or [`Refused`]. A refusal names what is
//!   missing, because a sound that does not happen with no explanation is the hardest bug in a game to find.
//! - [`SirenSettings`] — the mix table: the buses ([`DEFAULT_BUSES`] out of the box), their levels, mutes,
//!   voice caps and the ducks between them.
//! - [`SirenMix`] — this frame's mix, readable: one number per bus, master and ducks included, plus the live
//!   voice count per bus. The editor's audio panel draws it and a game's own meter reads it.
//! - [`MusicScore`] with [`MusicLayers`] and [`Stinger`] — vertical layering, moved on the next beat or bar.
//! - [`SirenLog`] — the last 256 decisions, each with the real time it happened.
//!
//! # Features
//!
//! - `serde` — `Serialize`/`Deserialize` on the cue table, the mix table and the score, with `from_ron` and
//!   `to_ron`: a project ships its sound as data and the Cauldron edits it in
//!   `<project>/family/bevy_siren.ron`.
//! - `mixer` — the crate's own mixer on `rodio`: one summing graph for the game, where a bus's gain is one
//!   number the frame writes and the audio thread does the summing. Without it the crate plays through Bevy's
//!   `AudioPlayer` and applies bus gain per voice. **Both answer [`Played`], [`Refused`] and [`Voice`]
//!   identically**; only the sample path differs.
//!
//! # What this crate does not own
//!
//! The output device (Bevy's `AudioPlugin` opens it), the game's own state and pausing, and the sounds
//! themselves: the crate never opens a file, it maps the keys a project names
//! ([`SirenSounds`]). A game's positional data belongs to the game: a [`Play`] takes a [`Vec3`] and the
//! listener is Bevy's `SpatialListener`.
//!
//! # Units
//!
//! Everything a person writes is decibels, seconds and metres. `dB <= −80` is silence
//! ([`SILENT_DB`](model::SILENT_DB)), pitch is semitones, distances are metres, and every gain moves at
//! [`MIX_TAU_S`](model::MIX_TAU_S) so nothing steps audibly.

#![warn(missing_docs)]

use bevy::prelude::*;

pub mod cue;
pub mod library;
pub mod log;
pub mod mix;
pub mod model;
pub mod music;
mod ogg;
pub mod output;
pub mod play;
pub mod playback;
pub mod sounds;

/// Semantic selection, behind the `semantic` feature.
#[cfg(feature = "semantic")]
pub mod semantic;

#[cfg(feature = "mixer")]
pub mod mixer;

pub use cue::{Attenuation, Cue, CueProblem, CueTable, Mode, PitchRange, Repeat, SoundSlot};
pub use log::{Entry, SirenLog};
pub use mix::{
    BusLevel, DEFAULT_BUSES, Duck, DuckClock, SETTINGS_CHECKS, SirenMix, SirenSettings, Stopping,
    UNSTEALABLE_PRIORITY,
};
pub use music::{MUSIC_CHECKS, MusicLayer, MusicLayers, MusicScore, MusicState, MusicVoice, Stinger, When};
pub use output::{AudioOutput, Backend, Output};
pub use play::{
    Compensation, Completes, MixerVoice, Play, Played, Refusal, Refused, Samples, SirenChoices, StopCue,
    StopVoice, Voice,
};
pub use playback::start_pending;
pub use sounds::SirenSounds;

/// The crate's systems, in the order they run.
///
/// A host that needs to slot its own work between them does it by set: `before(Choose)` to decide what to play,
/// `after(Play)` to react to what did, `after(Mix)` to read a mix that has settled.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SirenSystems {
    /// Decide what to play: the music plugin's layer changes, a game's own ambience logic.
    Choose,
    /// Turn requests into voices or refusals.
    Play,
    /// Bus levels, ducks, every voice's volume, and the retiring of finished voices.
    Mix,
}

/// The cue table, the mix, playback and the log. A game adds this and one line per sound.
///
/// It never adds [`AudioPlugin`](bevy::audio::AudioPlugin): a game's `DefaultPlugins` has it, and adding it twice
/// panics. A host without `DefaultPlugins` (a test) adds nothing — every system works without a device, and a
/// voice is still a [`Voice`] with a [`Played`] behind it.
pub struct SirenPlugin;

/// Layered music and stingers: vertical layering that moves on the bar. Needs [`SirenPlugin`].
pub struct MusicPlugin;

/// `(SirenPlugin, MusicPlugin)`: the two, in the order the music needs.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_siren::*;
/// App::new().add_plugins((DefaultPlugins, SirenPlugins));
/// ```
pub struct SirenPlugins;

impl PluginGroup for SirenPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>().add(SirenPlugin).add(MusicPlugin)
    }
}

impl Plugin for SirenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CueTable>()
            .init_resource::<SirenSounds>()
            .init_resource::<SirenSettings>()
            .init_resource::<SirenMix>()
            .init_resource::<SirenLog>()
            .init_resource::<DuckClock>()
            .init_resource::<SirenChoices>()
            // A headless app (`MinimalPlugins`) has no `TimePlugin`, and every system here reads real time. The
            // init is idempotent: a full app's `TimePlugin` already inserted it and this changes nothing.
            .init_resource::<Time<Real>>()
            .init_resource::<Backend>()
            .add_message::<Play>()
            .add_message::<Played>()
            .add_message::<Refused>()
            .add_message::<StopVoice>()
            .add_message::<StopCue>()
            .configure_sets(Update, (SirenSystems::Choose, SirenSystems::Play, SirenSystems::Mix).chain())
            .add_systems(
                Update,
                (
                    // The seed is the mix table's, and it must settle before anything draws from the stream: the music
                    // starts layers in `Choose` too, and a seed change on that frame should reach them.
                    play::seed_choices.in_set(SirenSystems::Choose),
                    (
                        play::play_requests,
                        playback::start_pending,
                        playback::trim_ogg_loops,
                        playback::refuse_failed_loads,
                    )
                        .chain()
                        .in_set(SirenSystems::Play),
                    (mix::update_mix, mix::apply_stopping, mix::retire_voices, mix::stop_requests)
                        .chain()
                        .in_set(SirenSystems::Mix),
                    // **After everything that may despawn a voice**, so the removal it reports is this frame's. `Mix` is
                    // where the crate's own despawns are commanded; Bevy applies them before the next frame's `Update`,
                    // which is when this reads them.
                    mix::log_despawned_voices.after(SirenSystems::Mix),
                ),
            );

        #[cfg(feature = "mixer")]
        if let Some(backend) = crate::mixer::MixerBackend::open() {
            app.insert_resource(Output(Box::new(backend)));
            app.insert_resource(Backend(true));
        }
    }
}

impl Plugin for MusicPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicState>().add_message::<MusicLayers>().add_message::<Stinger>().add_systems(
            Update,
            (
                music::start_layers.in_set(SirenSystems::Choose),
                music::adopt_layers.after(SirenSystems::Play).before(SirenSystems::Mix),
                // A stinger's duck is armed when its play is answered, and before the mix reads the clock, so the dip
                // lands on the frame the stinger fires rather than a frame later.
                music::arm_stinger_ducks.after(SirenSystems::Play).before(SirenSystems::Mix),
                music::move_layers.in_set(SirenSystems::Mix).before(mix::update_mix),
                music::play_stingers.in_set(SirenSystems::Choose),
            ),
        );
    }
}

#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
