//! Shared harness for the tests: a headless app, message draining, and real-time advance.
//!
//! Cargo links `tests/all.rs` as the crate root and this file as a module of it, so every test file uses
//! `crate::helpers::…`. A helper here is used by two or more files; one used by one file stays there.

use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use bevy_siren::{
    CueTable, MusicPlugin, Played, Refused, SirenPlugin, SirenSettings, SirenSounds, Stopping, Voice,
};

/// A headless app with no audio device: `MinimalPlugins` has no `AudioPlugin`, so nothing opens an output and every
/// assertion is on the crate's own decision — `Played`, `Refused`, `Voice` and the mix.
///
/// `TimePlugin` runs on [`TimeUpdateStrategy::Automatic`], which reads the wall clock and would overwrite the manual
/// advance [`advance`] makes; `ManualDuration` is the clock these tests drive.
pub fn app(table: CueTable, settings: SirenSettings, sounds: SirenSounds, music: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    if music {
        app.add_plugins((SirenPlugin, MusicPlugin));
    } else {
        app.add_plugins(SirenPlugin);
    }
    app.insert_resource(table);
    app.insert_resource(settings);
    app.insert_resource(sounds);
    app
}

/// Handles for every key, each the default one: a play names a key and the asset's bytes are never read here.
pub fn sounds(keys: &[&str]) -> SirenSounds {
    let mut sounds = SirenSounds::new();
    for key in keys {
        sounds.insert(*key, Handle::<AudioSource>::default());
    }
    sounds
}

/// The chi-square statistic of `counts` against an even split: Σ (observed − expected)² / expected, with `counts.len() − 1` degrees of freedom.
///
/// The seed tests compare it with the **upper 0.1 % point** of its distribution (10.83, 13.82, 16.27, 24.32 and 37.70 for 1, 2, 3, 7 and 15 degrees of freedom; `docs/loop/runs/2026-11-04-121/evidence/after/seed-probe-after.txt`, section 0,
/// prints the upper tail at each and it reads 0.00100). The seeds are fixed, so a result is deterministic: it clears the bound or it does not.
pub fn chi_square(counts: &[usize]) -> f64 {
    let total: usize = counts.iter().sum();
    let expected = total as f64 / counts.len() as f64;
    counts.iter().map(|&observed| (observed as f64 - expected).powi(2) / expected).sum()
}

/// Every [`Played`] written since the last call.
pub fn played(app: &mut App) -> Vec<Played> {
    app.world_mut().resource_mut::<Messages<Played>>().drain().collect()
}

/// Every [`Refused`] written since the last call.
pub fn refused(app: &mut App) -> Vec<Refused> {
    app.world_mut().resource_mut::<Messages<Refused>>().drain().collect()
}

/// How many voices the world holds, including the ones fading out.
pub fn voices(app: &mut App) -> usize {
    app.world_mut().query::<&Voice>().iter(app.world()).count()
}

/// How many voices a player can still hear: the ones not already stopping.
pub fn live_voices(app: &mut App) -> usize {
    app.world_mut()
        .query::<(&Voice, Option<&Stopping>)>()
        .iter(app.world())
        .filter(|(_, s)| s.is_none())
        .count()
}

/// Every live voice's cue name.
pub fn voice_cues(app: &mut App) -> Vec<String> {
    app.world_mut().query::<&Voice>().iter(app.world()).map(|v| v.cue.clone()).collect()
}

/// Sets the clock every later update advances real time by: the mix and the music read `Time<Real>`, and a headless
/// app's `TimePlugin` only moves it on [`TimeUpdateStrategy`].
pub fn advance(app: &mut App, seconds: f64) {
    app.world_mut().insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(seconds)));
}

/// Runs updates of `frame_s` each until `seconds` have passed, so a gain can ease and a duck can recover.
pub fn run_for(app: &mut App, seconds: f64, frame_s: f64) {
    let frames = (seconds / frame_s).ceil() as usize;
    advance(app, frame_s);
    for _ in 0..frames {
        app.update();
    }
}

/// A real 16-bit mono WAV of 64 samples: long enough for a decoder to accept, short enough to end at once.
///
/// Only `tests/mixer.rs` uses it, and that module is behind the `mixer` feature, so a build without the feature would
/// otherwise warn that this is dead.
#[cfg(feature = "mixer")]
pub fn tiny_wav() -> Vec<u8> {
    const RATE: u32 = 22_050;
    let count = 64usize;
    let mut out = Vec::with_capacity(44 + count * 2);
    let data_len = (count * 2) as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.3;
        out.extend_from_slice(&((sample * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}
