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

use crate::cue::CueTable;
use crate::log::{Entry, SirenLog};
use crate::output::Output;
use crate::play::{MixerVoice, Refusal, Refused, Samples, Voice};

/// The `sound` key a cue's chosen variant names, or `None` when the table can no longer answer for it.
fn key_of(table: &CueTable, cue: &str, variant: usize) -> Option<String> {
    table.get(cue)?.sounds.get(variant).map(|slot| slot.sound.clone())
}

/// Starts every waiting voice on the crate's mixer, once its asset has loaded.
///
/// Runs in [`SirenSystems::Play`](crate::SirenSystems) after the requests, so a play whose bytes are already in
/// memory starts in the frame it was asked for. Without an [`Output`] — the default build, or one whose device
/// failed to open — this system returns at once and Bevy's `AudioPlayer` owns every voice.
#[allow(clippy::too_many_arguments)]
pub fn start_pending(
    mut commands: Commands,
    output: Option<ResMut<Output>>,
    assets: Option<Res<Assets<AudioSource>>>,
    table: Res<CueTable>,
    waiting: Query<(Entity, &Voice, &Samples), Without<MixerVoice>>,
    // **Every voice the mixer is playing right now**, which is what `waiting` is not: a started voice carries a
    // [`MixerVoice`] and the query above skips it.
    playing: Query<(Entity, &MixerVoice, &Voice)>,
    // **The starts this system handed out, kept where the entity cannot take them with it.** A voice's mixer number
    // lives in a component on the voice entity, so despawning that entity — which is what happens when a game despawns
    // the parent a `Play::on` sound belongs to — destroys the number and leaves the `rodio` player sounding with its slot
    // held for ever. Measured: one voice started, the owner despawned, `stop_voice` **never called**.
    mut starts: Local<Vec<(Entity, String, usize)>>,
    mut refused: MessageWriter<Refused>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
) {
    let (Some(mut output), Some(assets)) = (output, assets) else {
        return;
    };
    // **Stop whatever the world has taken.** A start whose entity is not among `playing` was despawned by its owner;
    // its slot must be freed, exactly as `retire_voices` frees one whose sample ended. One retain over the starts this
    // system made, and no per-frame allocation beyond the list it already keeps.
    starts.retain(|(entity, bus, index)| {
        if playing.get(*entity).is_ok() {
            return true;
        }
        output.0.stop_voice(bus, *index);
        false
    });
    let now = real.elapsed_secs_f64();
    for (entity, voice, samples) in &waiting {
        let Some(source) = assets.get(&samples.0) else {
            // The asset is still loading: the voice waits, exactly as Bevy's `AudioPlayer` would.
            continue;
        };
        let looping = voice.mode.is_loop();
        // The mixer's own gain stage starts at the voice's own gain, and the bus's number is written separately by
        // `update_mix`; a duck is therefore one store per bus. The mix keeps this stage current — a music layer
        // moving on the bar, a stop fading out — which is why `update_mix` records what it wrote in `Applied`.
        let started = output.0.play(
            &voice.bus,
            &source.bytes,
            voice.base_db,
            crate::model::semitones_to_ratio(voice.semitones),
            looping,
        );
        match started {
            Some(index) => {
                starts.push((entity, voice.bus.clone(), index));
                commands.entity(entity).insert((MixerVoice(index), crate::play::Completes::default()));
            }
            None => {
                // **Name the sound the decoder refused, not the cue that asked for it.** `MissingSound::sound` is
                // documented as the key a project maps to a file, and the cue's name is not one: a broken `step_03.wav`
                // must send a person to `step_03`, not to a `footstep.grass` that never existed as a file. The key is
                // the one `answer` chose, recovered from the same cue and variant; a table edited between the play and
                // the load can no longer answer, and then the cue really is unknown.
                let reason = match key_of(&table, &voice.cue, voice.variant) {
                    Some(sound) => Refusal::MissingSound { sound },
                    None => Refusal::UnknownCue,
                };
                log.push(Entry::Refused { at_s: now, cue: voice.cue.clone(), reason: reason.to_string() });
                refused.write(Refused { cue: voice.cue.clone(), reason });
                commands.entity(entity).despawn();
            }
        }
    }
}
