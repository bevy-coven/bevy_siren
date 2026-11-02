//! **Where a gain goes**, and the one type that lets the two backends share every system.
//!
//! With the `mixer` feature on, the crate sums one graph and writes a bus's gain as one number; with it off,
//! Bevy's `AudioPlayer` owns playback and a voice's gain goes into its own `AudioSink`. Those are different
//! *tables*, and the crate does not want two copies of every system to differ only in which table a number is
//! written to.
//!
//! So the mix talks to a trait object. [`SirenPlugin`](crate::SirenPlugin) inserts an [`Output`] only when the
//! `mixer` feature is on; a system asks for `Option<ResMut<Output>>` and, on `None`, writes the same number into
//! a `AudioSink` instead. Nothing is stubbed: `None` means *Bevy owns playback*, which is a real path and the
//! default one.
//!
//! The virtual call is once per bus per frame and once per voice whose gain moved more than 0.01 dB, so it costs
//! no more than the `if` it replaces.

use std::sync::Arc;

use bevy::prelude::*;

/// A place a gain can be written: the crate's own mixer, when the `mixer` feature built one.
///
/// Implemented by [`MixerBackend`](crate::mixer::MixerBackend) only, and only with the feature on. A game never
/// implements it and never names it: it is inserted into the world by the plugin, and a system only asks whether
/// one exists.
pub trait AudioOutput: Send + Sync + 'static {
    /// Writes `bus`'s gain in dB.
    fn set_bus_db(&mut self, bus: &str, db: f32);

    /// Writes one voice's own gain in dB.
    fn set_voice_db(&mut self, bus: &str, voice: usize, db: f32);

    /// Stops a voice.
    fn stop_voice(&mut self, bus: &str, voice: usize);

    /// Starts a sample on `bus`, returning the voice's index, or `None` when the sample cannot be decoded or
    /// there is no device.
    fn play(&mut self, bus: &str, bytes: &Arc<[u8]>, db: f32, speed: f32, looping: bool) -> Option<usize>;

    /// True when a voice's sample has ended or it was stopped.
    fn finished(&self, bus: &str, voice: usize) -> bool;
}

/// Which backend owns playback in this world.
///
/// A [`Resource`] that always exists, so a system can ask without an `Option`, and so a game can read which path is
/// live: `true` means the crate's own mixer started and every voice goes through it, `false` means Bevy's
/// `AudioPlayer` owns playback. Inserted by [`SirenPlugin`](crate::SirenPlugin).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Backend(pub bool);

/// The world's output, when this build has one.
///
/// A [`Resource`] holding a [`Box`] rather than the concrete mixer type, so a system's signature does not change
/// with the feature.
#[derive(Resource)]
pub struct Output(pub Box<dyn AudioOutput>);
