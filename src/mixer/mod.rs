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
//! **A loop repeats the sample and *then* applies the gain.** `rodio`'s `repeat_infinite` stores what it is given and replays the stored values on
//! every later pass, so a gain applied *inside* the repeat is the gain of the first pass for ever: a layer that starts silent would never come in, and a
//! fade would be a click, with nothing refused or logged. The voice's gain is read for every sample *after* the repeat (`looped`), and a test pulls
//! three passes through a real `rodio::Player` to hold it there. What `Repeat` costs is that the decoded loop stays in memory as `f32` for the life of
//! the voice — about 23 MB a minute of 48 kHz stereo; Bevy's path does the same.
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
        let player = connect(&track.mixer, decoder, &gain, speed, looping);

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

/// **A voice that plays once**: the decoded sample, with the voice's gain applied to each sample as it is pulled.
fn once<S: Source + Send + 'static>(sample: S, gain: Arc<AtomicU32>) -> impl Source + Send + 'static {
    VoiceGain { source: sample, gain }
}

/// **A looping voice**: the sample repeated, and *then* the voice's gain.
///
/// The order is the whole point. `rodio`'s `repeat_infinite` **stores** what it is given — a span of up to 32 768 samples at a time, pulled out of
/// the source in one go — and replays the stored values on every later pass (`rodio-0.22.2/src/source/{repeat,buffered}.rs`). A gain applied *inside*
/// the repeat is therefore the gain of the instant the first pass was decoded, **for ever**: a music layer that starts silent never comes in, a fade
/// is a click, and nothing refuses or logs, because `update_mix` did store the number. Applied *outside* it, the cache holds the sample at unity and
/// the gain is read for every sample as it goes to the player, which is what a gain is.
fn looped<S: Source + Send + 'static>(sample: S, gain: Arc<AtomicU32>) -> impl Source + Send + 'static {
    VoiceGain { source: sample.repeat_infinite(), gain }
}

/// **Connects a voice to its bus**: a `rodio::Player` on the bus's mixer, playing the decoded sample at `speed`, once or as a loop, with the voice's gain. It is everything `play` does to
/// make a sound except the slot bookkeeping, so the tests below build a voice the way the backend does.
fn connect<S: Source + Send + 'static>(
    mixer: &Mixer,
    sample: S,
    gain: &Arc<AtomicU32>,
    speed: f32,
    looping: bool,
) -> Player {
    let player = Player::connect_new(mixer);
    player.set_volume(1.0);
    player.set_speed(speed.clamp(1.0 / 16.0, 16.0));
    if looping {
        player.append(looped(sample, gain.clone()));
    } else {
        player.append(once(sample, gain.clone()));
    }
    player
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::num::NonZero;

    use rodio::buffer::SamplesBuffer;

    use super::*;

    /// How long the test sample is, in samples: **more than one `Buffered` span (32 768)**, so a pass of the loop is two spans, and a gain that is frozen
    /// at the first span's value and a gain that is merely late both show.
    const LEN: usize = 40_000;

    /// A mono sample that is `1.0` throughout, so that every sample a voice produces *is* the gain that was in force when it was pulled.
    fn ones() -> SamplesBuffer {
        SamplesBuffer::new(NonZero::new(1).unwrap(), NonZero::new(48_000).unwrap(), vec![1.0; LEN])
    }

    fn gain_of(value: f32) -> Arc<AtomicU32> {
        Arc::new(AtomicU32::new(value.to_bits()))
    }

    fn set(gain: &AtomicU32, value: f32) {
        gain.store(value.to_bits(), Ordering::Relaxed);
    }

    fn pull(voice: &mut impl Iterator<Item = f32>, n: usize) -> Vec<f32> {
        voice.by_ref().take(n).collect()
    }

    /// How many of `samples` are not `expected`.
    fn wrong(samples: &[f32], expected: f32) -> usize {
        samples.iter().filter(|s| **s != expected).count()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt()
    }

    /// A 16-bit PCM WAV, as bytes: `frames` of a sine at `hz`, peak `peak`, the same on every channel.
    fn wav(channels: u16, rate: u32, frames: usize, hz: f32, peak: f32) -> Vec<u8> {
        let mut data = Vec::with_capacity(frames * usize::from(channels) * 2);
        for n in 0..frames {
            let sample = ((n as f32 * hz * std::f32::consts::TAU / rate as f32).sin() * peak * 32_767.0)
                .round() as i16;
            for _ in 0..channels {
                data.extend_from_slice(&sample.to_le_bytes());
            }
        }
        let mut wav = Vec::with_capacity(44 + data.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&rate.to_le_bytes());
        wav.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
        wav.extend_from_slice(&(channels * 2).to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        wav
    }

    /// **The bug.** Pulled across three wraps, with the gain moved at the start of a pass and in the middle of one, every sample is the gain in force when
    /// it was pulled. The unfixed order replayed the first pass's gain on every later pass.
    #[test]
    fn a_looping_voice_follows_its_gain_on_every_pass() {
        let gain = gain_of(1.0);
        let mut voice = looped(ones(), gain.clone());

        assert_eq!(wrong(&pull(&mut voice, LEN), 1.0), 0, "pass 1 is the sample at unity");

        set(&gain, 0.5);
        let second = pull(&mut voice, LEN);
        assert_eq!(wrong(&second, 0.5), 0, "pass 2: samples still carrying the first pass's gain, of {LEN}");

        set(&gain, 0.25);
        let first_half = pull(&mut voice, LEN / 2);
        set(&gain, 2.0);
        let second_half = pull(&mut voice, LEN / 2);
        assert_eq!(wrong(&first_half, 0.25), 0, "pass 3, first half");
        assert_eq!(wrong(&second_half, 2.0), 0, "pass 3, second half: a gain moved in the middle of a pass");

        set(&gain, 0.0);
        assert_eq!(
            wrong(&pull(&mut voice, LEN), 0.0),
            0,
            "a voice moved to silence is silent on the next pass, not the sample"
        );
    }

    /// **A gain stored between two samples is on the next one**, on the first pass as on every other: the voice reads the number per sample. The unfixed order
    /// delivered a change inside the first pass only at the next span boundary, up to 32 768 samples later.
    #[test]
    fn a_gain_stored_between_two_samples_is_on_the_next_one() {
        fn check(mut voice: impl Iterator<Item = f32>, gain: &AtomicU32, what: &str) {
            assert_eq!(wrong(&pull(&mut voice, 100), 1.0), 0, "{what}: before the store");
            set(gain, 0.5);
            assert_eq!(pull(&mut voice, 1), [0.5], "{what}: the very next sample carries the new gain");
            assert_eq!(
                wrong(&pull(&mut voice, 20_000), 0.5),
                0,
                "{what}: and so does everything after it, inside the same span"
            );
        }
        let (a, b) = (gain_of(1.0), gain_of(1.0));
        check(once(ones(), a.clone()), &a, "plays once");
        check(looped(ones(), b.clone()), &b, "loops");
    }

    /// The control: a voice that plays once never had the problem, and keeps following its gain to the last sample, then ends.
    #[test]
    fn a_voice_that_plays_once_follows_its_gain_and_ends() {
        let gain = gain_of(1.0);
        let mut voice = once(ones(), gain.clone());
        assert_eq!(wrong(&pull(&mut voice, 10_000), 1.0), 0);
        set(&gain, 0.25);
        let rest = pull(&mut voice, LEN);
        assert_eq!(rest.len(), LEN - 10_000, "a one-shot ends with its sample");
        assert_eq!(wrong(&rest, 0.25), 0);
    }

    /// The same through what the crate really builds: a `rodio::Player` on a bus's `Mixer`, pulled offline — the samples the device would be handed.
    #[test]
    fn a_looping_voice_through_a_player_and_a_bus_mixer_follows_its_gain() {
        let (mixer, mut bus) = rodio::mixer::mixer(NonZero::new(1).unwrap(), NonZero::new(48_000).unwrap());
        let gain = gain_of(1.0);
        let _player = connect(&mixer, ones(), &gain, 1.0, true);

        let first = pull(&mut bus, LEN);
        let latency = first.iter().take_while(|s| **s == 0.0).count();
        assert!(
            latency < 1_024,
            "the player's first sample arrives after at most a block of silence: {latency}"
        );
        assert_eq!(wrong(&first[latency..], 1.0), 0, "pass 1 at unity");

        set(&gain, 0.25);
        let second = pull(&mut bus, LEN);
        assert_eq!(wrong(&second, 0.25), 0, "pass 2 through the player and the mixer, of {LEN}");

        set(&gain, 0.0);
        assert_eq!(
            wrong(&pull(&mut bus, LEN), 0.0),
            0,
            "a layer moved to silence is silent on the next pass"
        );
    }

    /// **A stem that is not in the bus's format** — stereo 44.1 kHz on a 48 kHz stereo bus, the ordinary case — is converted inside the mixer and must still follow its gain on every
    /// pass. The test above runs in the bus's own format, where nothing converts; this one goes through a real `Decoder` on real WAV bytes and the mixer's resampler.
    #[test]
    fn a_stem_in_another_rate_follows_its_gain_through_the_bus() {
        let (mixer, mut bus) = rodio::mixer::mixer(NonZero::new(2).unwrap(), NonZero::new(48_000).unwrap());
        let gain = gain_of(1.0);
        // 0.2 s of 1 kHz — 200 whole cycles, so the loop is seamless — decoded by the same `Decoder` `play` uses.
        let stem =
            rodio::Decoder::new(Cursor::new(wav(2, 44_100, 8_820, 1_000.0, 0.25))).expect("a WAV decodes");
        let _player = connect(&mixer, stem, &gain, 1.0, true);

        // Three seconds of stereo: fifteen passes of the loop.
        let at_unity = rms(&pull(&mut bus, 2 * 144_000));
        set(&gain, 0.25);
        let _settling = pull(&mut bus, 2 * 1_024); // the resampler's own latency, a few frames
        let quartered = rms(&pull(&mut bus, 2 * 144_000));
        assert!(
            (quartered / at_unity - 0.25).abs() < 0.01,
            "a quarter of the gain is a quarter of the level: {at_unity} then {quartered}"
        );

        set(&gain, 0.0);
        let _settling = pull(&mut bus, 2 * 1_024);
        assert!(
            rms(&pull(&mut bus, 2 * 144_000)) < 1e-6,
            "a layer moved to silence is silent on every pass that follows"
        );
    }

    /// **A loop of nothing ends.** An empty export is an authoring mistake a game will meet, and `Repeat` must hand the player an end rather than ask an empty sample for ever: the voice
    /// reports finished (so its slot is freed) and the bus hears silence, with no spin on the audio thread.
    #[test]
    fn a_loop_of_an_empty_sample_ends_instead_of_repeating_nothing() {
        let (mixer, mut bus) = rodio::mixer::mixer(NonZero::new(1).unwrap(), NonZero::new(48_000).unwrap());
        let gain = gain_of(1.0);
        let empty = rodio::Decoder::new(Cursor::new(wav(1, 48_000, 0, 440.0, 0.25)))
            .expect("a WAV with no frames decodes");
        let player = connect(&mixer, empty, &gain, 1.0, true);
        let heard = pull(&mut bus, 4_800);
        assert_eq!(wrong(&heard, 0.0), 0, "nothing to repeat is silence");
        assert!(player.empty(), "the voice has ended, so `finished` is true and its slot is freed");
    }
}
