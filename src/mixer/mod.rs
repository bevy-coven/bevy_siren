//! **Our own mixer on rodio**: one summing graph for the whole game, with a bus's gain as one number the
//! frame writes.
//!
//! Bevy's default audio path creates one `Player` per voice and mixes them inside the device. That is fine,
//! and it is what this crate uses unless the [`mixer`](crate#features) feature is on. What it cannot do is
//! *sum a bus*: with it, a bus's level is applied per voice, so twenty footsteps are twenty multiplies on the
//! game thread and a duck is twenty writes rather than one.
//!
//! This backend builds the graph a game mixer builds (Böttcher 2008; Gregory §14.5.8's categories): one
//! [`rodio`] `Mixer` per bus, each fed into the device through a `BusGain` source that multiplies every
//! sample by the bus's current gain, read from an `AtomicU32`. A frame that moves a bus writes one number;
//! the audio thread does the summing. A voice is a [`rodio::Player`] connected to its bus's mixer, with its own
//! gain as a second atomic, so a duck is one store per bus and nothing per voice.
//!
//! **A voice ends here the way it ends on Bevy's path.** There is no `AudioSink` to ask, so
//! [`finished`](AudioOutput::finished) reads the `rodio` player: a once-voice whose source has produced its last
//! sample reports finished, and [`retire_voices`](crate::mix::retire_voices) despawns it and frees its slot in the
//! budget. A voice that kept its slot after it stopped sounding would turn a full bus into a permanent lockout.
//!
//! # What this backend does not do
//!
//! It does not pan. Bevy's spatial path is a two-ear [`rodio::SpatialPlayer`], and a rodio source cannot be
//! re-positioned once it is inside a `Player`, so a spatial cue's [`Attenuation`](crate::cue::Attenuation) is
//! applied as a gain at play time — correct at the moment of the play, and not tracking a moving emitter
//! afterwards. A game that needs a moving emitter's pan wants Bevy's path, which is the default.
//!
//! # Failing like Bevy fails
//!
//! [`MixerBackend::open`] returns `None` and warns `No audio device found.` — the same sentence Bevy's own path
//! prints — when there is no output device, and the plugin then inserts no [`Output`](crate::output::Output), so voices
//! fall back to Bevy's `AudioPlayer`. A headless machine runs with the feature on and hears nothing.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use bevy::prelude::*;
use rodio::mixer::Mixer;
use rodio::source::Source;
use rodio::{ChannelCount, MixerDeviceSink, Player, Sample, SampleRate};

use crate::output::AudioOutput;

/// The one graph: the device, and one track per bus.
///
/// Built by [`SirenPlugin`](crate::SirenPlugin) and reachable in the world as
/// [`Output`](crate::output::Output). A game never builds one.
#[derive(Resource)]
pub struct MixerBackend {
    device: MixerDeviceSink,
    buses: HashMap<String, BusTrack>,
    format: (ChannelCount, SampleRate),
}

impl std::fmt::Debug for MixerBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MixerBackend")
            .field("buses", &self.buses.keys().collect::<Vec<_>>())
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

struct BusTrack {
    mixer: Mixer,
    gain: Arc<AtomicU32>,
    /// **`None` is a free slot**, and the length is the peak *concurrent* voice count — never the number of sounds
    /// played. `play` reuses the first free slot; the alternative, appending, retains one `rodio` player and its
    /// queues for every sound a game has ever played.
    slots: Vec<Option<VoiceTrack>>,
    /// Each slot's generation, bumped when the slot is handed out. Together with the slot it forms the voice index, so
    /// a stale index addresses nothing rather than the slot's next occupant.
    generations: Vec<u32>,
}

struct VoiceTrack {
    player: Player,
    gain: Arc<AtomicU32>,
    stopped: bool,
}

impl MixerBackend {
    /// Opens the default output device, or returns `None` with Bevy's own warning.
    pub fn open() -> Option<Self> {
        let mut device = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(device) => device,
            Err(_) => {
                warn!("No audio device found.");
                return None;
            }
        };
        device.log_on_drop(false);
        let config = device.config();
        Some(Self { buses: HashMap::new(), format: (config.channel_count(), config.sample_rate()), device })
    }

    /// The format every bus is built in: the device's own, so nothing resamples on the bus path.
    pub fn format(&self) -> (ChannelCount, SampleRate) {
        self.format
    }

    /// The bus `name` is at, in linear gain, for a test or a panel that reads the graph back.
    pub fn bus_gain(&self, name: &str) -> Option<f32> {
        self.buses.get(name).map(|t| f32::from_bits(t.gain.load(Ordering::Relaxed)))
    }

    /// How many **slots** `name` holds: the peak number of voices it has had at once, not the number it has played.
    pub fn voices(&self, name: &str) -> usize {
        self.buses.get(name).map_or(0, |t| t.slots.len())
    }

    /// The voice index `slot` is at when handed out with `generation`, or `None` when the slot is free or the
    /// generation is not the one the slot is holding. Every index-taking method goes through this, so a stale name is
    /// a no-op in one place rather than a rule each method remembers.
    fn occupant_mut(&mut self, bus: &str, voice: usize) -> Option<&mut VoiceTrack> {
        let (slot, generation) = crate::model::unpack_voice(voice);
        let track = self.buses.get_mut(bus)?;
        if track.generations.get(slot).copied() != Some(generation) {
            return None;
        }
        track.slots.get_mut(slot).and_then(Option::as_mut)
    }

    fn bus_mut(&mut self, name: &str) -> &mut BusTrack {
        let (channels, rate) = self.format;
        if !self.buses.contains_key(name) {
            let (mixer, output) = rodio::mixer::mixer(channels, rate);
            let gain = Arc::new(AtomicU32::new(1.0f32.to_bits()));
            self.device.mixer().add(BusGain { source: output, gain: gain.clone() });
            self.buses.insert(
                name.to_owned(),
                BusTrack { mixer, gain, slots: Vec::new(), generations: Vec::new() },
            );
        }
        self.buses.get_mut(name).expect("just inserted")
    }
}

impl AudioOutput for MixerBackend {
    fn set_bus_db(&mut self, bus: &str, db: f32) {
        let track = self.bus_mut(bus);
        track.gain.store(crate::model::db_to_gain(db).to_bits(), Ordering::Relaxed);
    }

    fn set_voice_db(&mut self, bus: &str, voice: usize, db: f32) {
        // **A stale index addresses nothing.** The generation in `voice` must match the slot's, or a gain meant for a
        // voice the world has not finished despawning would move its replacement.
        if let Some(entry) = self.occupant_mut(bus, voice) {
            entry.gain.store(crate::model::db_to_gain(db).to_bits(), Ordering::Relaxed);
        }
    }

    fn stop_voice(&mut self, bus: &str, voice: usize) {
        if let Some(entry) = self.occupant_mut(bus, voice) {
            entry.player.stop();
            entry.stopped = true;
        }
    }

    fn play(&mut self, bus: &str, bytes: &Arc<[u8]>, db: f32, speed: f32, looping: bool) -> Option<usize> {
        let decoder = rodio::Decoder::new(std::io::Cursor::new(bytes.to_vec())).ok()?;
        let gain = Arc::new(AtomicU32::new(crate::model::db_to_gain(db).to_bits()));
        let track = self.bus_mut(bus);
        let player = Player::connect_new(&track.mixer);
        player.set_volume(1.0);
        player.set_speed(speed.clamp(1.0 / 16.0, 16.0));
        let voiced = VoiceGain { source: decoder, gain: gain.clone() };
        if looping {
            player.append(voiced.repeat_infinite());
        } else {
            player.append(voiced);
        }

        // **Reuse the first slot whose voice has ended.** Appending would retain one `rodio` player and its queues for
        // every sound a game has ever played; the vector's length is the peak *concurrent* count, which the budget
        // already bounds. A slot is freed here rather than in `retire_voices`, so the trait stays unchanged and the
        // cost sits in the one method that already allocates a player.
        let free =
            track.slots.iter().position(|slot| slot.as_ref().is_none_or(|v| v.stopped || v.player.empty()));
        let slot = free.unwrap_or(track.slots.len());
        // The generation is bumped on every hand-out: a new voice in a reused slot is a new name.
        let generation = track.generations.get(slot).copied().unwrap_or(0).wrapping_add(1);
        if slot == track.slots.len() {
            track.slots.push(None);
            track.generations.push(generation);
        } else {
            track.generations[slot] = generation;
        }
        track.slots[slot] = Some(VoiceTrack { player, gain, stopped: false });
        Some(crate::model::pack_voice(slot, generation))
    }

    fn finished(&self, bus: &str, voice: usize) -> bool {
        let (slot, generation) = crate::model::unpack_voice(voice);
        let Some(track) = self.buses.get(bus) else {
            return true;
        };
        if track.generations.get(slot).copied() != Some(generation) {
            // A name that no longer exists has, as far as its holder is concerned, ended.
            return true;
        }
        track.slots.get(slot).is_none_or(|slot| slot.as_ref().is_none_or(|v| v.stopped || v.player.empty()))
    }
}

/// A bus's gain: multiplies every sample of everything summed into it by one number read from an atomic.
struct BusGain<S> {
    source: S,
    gain: Arc<AtomicU32>,
}

impl<S: Source> Iterator for BusGain<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.source.next()?;
        let gain = f32::from_bits(self.gain.load(Ordering::Relaxed));
        Some(sample * gain as Sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S: Source> Source for BusGain<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.source.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.source.try_seek(pos)
    }
}

/// A voice's gain, the same way: one atomic store moves one sound.
struct VoiceGain<S> {
    source: S,
    gain: Arc<AtomicU32>,
}

impl<S: Source> Iterator for VoiceGain<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.source.next()?;
        let gain = f32::from_bits(self.gain.load(Ordering::Relaxed));
        Some(sample * gain as Sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.source.size_hint()
    }
}

impl<S: Source> Source for VoiceGain<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> ChannelCount {
        self.source.channels()
    }

    fn sample_rate(&self) -> SampleRate {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), rodio::source::SeekError> {
        self.source.try_seek(pos)
    }
}
