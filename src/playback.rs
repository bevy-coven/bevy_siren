//! **Starting a voice on whichever backend owns playback**, and nothing else.
//!
//! Bevy's path needs no system: an entity with `AudioPlayer` starts itself, waiting for its asset exactly as it
//! waits for a file. The crate's own mixer has no such hook, so this module reads the bytes of each
//! [`Samples`] handle once they are loaded and starts the voice on its bus.
//!
//! **This is the only place a voice starts twice-checked.** A voice that has a [`MixerVoice`] is playing; a voice
//! with [`Samples`] and no `MixerVoice` is waiting. A sample the decoder refuses is reported as a `Refused` with
//! the key, so a project learns which file to deliver rather than hearing nothing.

use bevy::audio::AudioSource;
use bevy::prelude::*;

use crate::log::{Entry, SirenLog};
use crate::output::Output;
use crate::play::{MixerVoice, Refusal, Refused, Samples, Voice};

/// Starts every waiting voice on the crate's mixer, once its asset has loaded.
///
/// Runs in [`SirenSystems::Play`](crate::SirenSystems) after the requests, so a play whose bytes are already in
/// memory starts in the frame it was asked for. Without an [`Output`] — the default build, or one whose device
/// failed to open — this system returns at once and Bevy's `AudioPlayer` owns every voice.
pub fn start_pending(
    mut commands: Commands,
    output: Option<ResMut<Output>>,
    assets: Option<Res<Assets<AudioSource>>>,
    waiting: Query<(Entity, &Voice, &Samples), Without<MixerVoice>>,
    mut refused: MessageWriter<Refused>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
) {
    let (Some(mut output), Some(assets)) = (output, assets) else {
        return;
    };
    let now = real.elapsed_secs_f64();
    for (entity, voice, samples) in &waiting {
        let Some(source) = assets.get(&samples.0) else {
            // The asset is still loading: the voice waits, exactly as Bevy's `AudioPlayer` would.
            continue;
        };
        let looping = voice.mode.is_loop();
        // The mixer's own gain is the bus's alone: a voice's own gain is applied here, once, and the mix writes
        // the bus. So a voice starts at its cue's gain and the mix moves it only when the cue's gain changes.
        let started = output.0.play(
            &voice.bus,
            &source.bytes,
            voice.base_db,
            crate::model::semitones_to_ratio(voice.semitones),
            looping,
        );
        match started {
            Some(index) => {
                commands.entity(entity).insert((MixerVoice(index), crate::play::Completes::default()));
            }
            None => {
                let reason = Refusal::MissingSound { sound: voice.cue.clone() };
                log.push(Entry::Refused { at_s: now, cue: voice.cue.clone(), reason: reason.to_string() });
                refused.write(Refused { cue: voice.cue.clone(), reason });
                commands.entity(entity).despawn();
            }
        }
    }
}
