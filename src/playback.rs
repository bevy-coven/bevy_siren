//! **Starting a voice on whichever backend owns playback**, and nothing else.
//!
//! Bevy's path needs no system to *start* a voice: an entity with `AudioPlayer` starts itself, waiting for its asset exactly
//! as it waits for a file. The crate's own mixer has no such hook, so this module reads the bytes of each
//! [`Samples`] handle once they are loaded and starts the voice on its bus. **The one thing Bevy's path cannot know** is how
//! long an Ogg loop is: `trim_ogg_loops` tells it, once the bytes are there.
//!
//! **This is the only place a voice starts twice-checked.** A voice that has a [`MixerVoice`] is playing; a voice
//! with [`Samples`] and no `MixerVoice` is waiting. A sample the decoder refuses is reported as a `Refused` with
//! the key, so a project learns which file to deliver rather than hearing nothing — and so is **a file that failed to
//! load** (`refuse_failed_loads`): the asset server knows, and a voice that waits for bytes that will never come is
//! `Played` for a sound that cannot sound, a slot held for ever, and a log that reads as healthy.

use bevy::audio::{AudioPlayer, AudioSink, AudioSource, PlaybackMode, PlaybackSettings, SpatialAudioSink};
use bevy::prelude::*;

use crate::cue::CueTable;
use crate::log::{Entry, SirenLog};
use crate::ogg;
use crate::output::Output;
use crate::play::{MixerVoice, Refusal, Refused, Samples, Voice};
use crate::sounds::{Files, cannot_load, failed_to_load};

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
    // **The slot is freed here; the *sentence* is not written here.** The mixer number lives in a component on the voice
    // entity, so a despawned owner destroys it and `stop_voice` is never called — measured before iteration 7. One
    // despawn, one line: `log_despawned_voices` covers **both** backends, and writing it in this `retain` too put
    // `owner despawned` in the log twice on the mixer, which the iteration-43 test caught.
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

/// The voices that have not started on either path: no mixer voice, no sink of either kind.
type Unstarted = (Without<MixerVoice>, Without<AudioSink>, Without<SpatialAudioSink>);

/// **Refuses a voice whose file failed to load, and frees it.** A play whose file had **already** failed is refused where it is asked (`play::answer`) and never spawns; this is the voice that was
/// spawned while the load was still in flight and whose load then failed — the play could not know. It answers as `start_pending` does for bytes the decoder refuses: `Refused { MissingSound { sound } }` and
/// a log line, here with the asset server's own cause and the path it looked for, and the voice is despawned so its budget slot is free at the next `update_mix`. The `Played` it already wrote stands; a
/// refusal after a `Played` is the second answer, and it is the true one.
///
/// **Only a voice that has not started and whose asset is absent is looked at**, so a running voice costs nothing (its archetype is skipped), and a load still `Loading` — or a handle the server has no
/// record of — is left to wait, exactly as before. Runs after `start_pending` and works on both paths: a voice has a [`MixerVoice`] only once the mixer started it, and a sink only once Bevy did.
pub(crate) fn refuse_failed_loads(
    mut commands: Commands,
    files: Files,
    table: Res<CueTable>,
    waiting: Query<(Entity, &Voice, &Samples), Unstarted>,
    mut refused: MessageWriter<Refused>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
) {
    let now = real.elapsed_secs_f64();
    for (entity, voice, samples) in &waiting {
        let Some(error) = cannot_load(&files, &samples.0) else {
            continue;
        };
        // The key the play chose, recovered as `start_pending` recovers it; a table edited since can no longer answer, and then the cue really is unknown.
        let (reason, sentence) = match key_of(&table, &voice.cue, voice.variant) {
            Some(sound) => {
                let sentence = failed_to_load(&sound, &error);
                (Refusal::MissingSound { sound }, sentence)
            }
            None => (Refusal::UnknownCue, Refusal::UnknownCue.to_string()),
        };
        log.push(Entry::Refused { at_s: now, cue: voice.cue.clone(), reason: sentence });
        refused.write(Refused { cue: voice.cue.clone(), reason });
        commands.entity(entity).despawn();
    }
}

/// Marks a voice whose loop length is settled — told, or found to need no telling — so [`trim_ogg_loops`] looks at each voice once.
#[derive(Component)]
pub(crate) struct LoopChecked;

/// The voices [`trim_ogg_loops`] has not looked at yet: Bevy's own (they carry an `AudioPlayer`), and not yet marked.
type Unchecked = (With<AudioPlayer>, Without<LoopChecked>);

/// **Tells Bevy's path how long an Ogg loop is.** `rodio`'s Vorbis decoders return the padding of an Ogg's last block (for 51 of Kenney's 254 files, with `lewton`) and Bevy loops what the decoder returns, so a loop of one is
/// longer than its stream by one padding at every wrap: a gap of near-silence or a repeated tail, and a stem that drifts off its bar. `PlaybackSettings::duration` is Bevy's own way to end a loop early
/// (`decoder.take_duration(duration).repeat_infinite()`), and the length is on the file's last page — see [`crate::ogg`].
///
/// A voice is looked at **once**. A loop whose asset is not loaded yet is looked at again next frame, which is still before Bevy can start it: Bevy waits for the same asset, in `PostUpdate`, after this
/// system's `Update` — **except in the one frame a system that runs after this one in `Update` adds the asset**, when Bevy starts the voice untrimmed and a later `duration` changes nothing. A one-shot is marked
/// without a read, because only a loop exposes the padding. A file that is not an Ogg, or is one the walk does not fully understand, is left exactly as it was, and so is a voice a host gave a `duration` of its own.
/// A voice the crate's mixer owns has no `AudioPlayer` and is not this system's: the mixer's own `play` does the same for its loop.
pub(crate) fn trim_ogg_loops(
    mut commands: Commands,
    assets: Option<Res<Assets<AudioSource>>>,
    mut voices: Query<(Entity, &Samples, &mut PlaybackSettings), Unchecked>,
) {
    let Some(assets) = assets else {
        return;
    };
    for (entity, samples, mut settings) in &mut voices {
        if matches!(settings.mode, PlaybackMode::Loop) {
            let Some(asset) = assets.get(&samples.0) else {
                continue; // not loaded yet: ask again next frame
            };
            if settings.duration.is_none()
                && let Some(length) = ogg::loop_length(&asset.bytes)
            {
                settings.duration = Some(length);
            }
        }
        commands.entity(entity).try_insert(LoopChecked);
    }
}
