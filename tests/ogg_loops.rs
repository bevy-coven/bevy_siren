//! **A loop of an Ogg sounds, and is as long as its stream declares.** Two things were wrong and both are held here. (1) With the `symphonia` Vorbis backend a decoder reports an empty span until its first sample, and
//! `repeat_infinite` of it — Bevy's own loop, `bevy_audio-0.20.1/src/audio_output.rs:165` — **produces no samples** (RustAudio/rodio#775): `the_duration_plays_exactly_…` fails if `Cargo.toml` selects it. (2) The decoders
//! return the padding of the last Vorbis block, so a loop of a decoded Ogg is longer than its stream by one padding at every wrap — a gap or a repeated tail, and a stem that drifts off its bar (`src/ogg.rs`,
//! `docs/loop/runs/2026-11-03-115/research.md`). Bevy's path can be told a loop's length without a decoder of the crate's own: `PlaybackSettings::duration` (`audio_output.rs:160`,
//! `decoder.take_duration(duration).repeat_infinite()`). These tests drive a headless app and read what the crate told the voice, with real Ogg files made by an encoder that is not this crate's
//! (`tests/semantic_decode.rs` says how).
//!
//! **Bevy's path is forced in every build, by removing the `Output`**: a `mixer` build on a machine with a device inserts one, and then the crate's own graph owns the voice — no `PlaybackSettings`, and `start_pending`
//! decodes the bytes, so a file the decoder refuses leaves no voice at all. A build with no device has no `Output`, which is the path these tests are about. (`Backend(false)` alone is not enough: `start_pending` does not read it.)

use std::sync::Arc;
use std::time::Duration;

use bevy::audio::{AudioSource, PlaybackSettings};
use bevy::prelude::*;
use bevy_siren::{Cue, CueTable, Mode, Output, Play, SirenSettings, SirenSounds, SoundSlot, Voice};

use crate::helpers::{app, sounds};

const TONE_OGG: &[u8] = include_bytes!("fixtures/tone.ogg");
const TONE_441_OGG: &[u8] = include_bytes!("fixtures/tone-441.ogg");
#[cfg(any(feature = "mixer", feature = "semantic"))]
const TONE_FLAC: &[u8] = include_bytes!("fixtures/tone.flac");
#[cfg(any(feature = "mixer", feature = "semantic"))]
const TONE_MP3: &[u8] = include_bytes!("fixtures/tone.mp3");

/// A headless app whose one cue, `"loop"`, plays the sound `"s"` in `mode`, on Bevy's path. `"s"` points at nothing loaded yet.
fn app_for(mode: Mode) -> App {
    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("s")], mode, ..Cue::new("loop") }]);
    let mut app = app(table, SirenSettings::default(), sounds(&["s"]), false);
    app.world_mut().remove_resource::<Output>();
    app
}

/// An app whose sound `"s"` is `bytes`, **already loaded**.
fn playing(bytes: &[u8], mode: Mode) -> App {
    let mut app = app_for(mode);
    let handle =
        app.world_mut().resource_mut::<Assets<AudioSource>>().add(AudioSource { bytes: Arc::from(bytes) });
    app.world_mut().resource_mut::<SirenSounds>().insert("s", handle);
    app
}

fn play(app: &mut App) {
    app.world_mut().write_message(Play::cue("loop"));
    app.update();
}

/// The `duration` the crate gave the voice's `PlaybackSettings`: the outer `Option` is *whether the voice is Bevy's*, the inner is the duration.
fn duration_of_the_voice(app: &mut App) -> Option<Option<Duration>> {
    app.world_mut()
        .query_filtered::<&PlaybackSettings, With<Voice>>()
        .iter(app.world())
        .next()
        .map(|settings| settings.duration)
}

/// **The control, and the point.** A looping Ogg whose bytes are already in memory is told its length in the frame it is played — before Bevy starts the sink in `PostUpdate`, which is when it reads it.
#[test]
fn a_looping_ogg_is_told_its_length_in_the_frame_it_is_played() {
    let mut app = playing(TONE_OGG, Mode::Loop);
    play(&mut app);
    let told = duration_of_the_voice(&mut app).expect("the voice is Bevy's, and has settings");
    assert!(told.is_some(), "a loop of a stream that declares its length is told it");
}

/// An asset that is not loaded yet has nothing to read, so the voice is left alone — and told the frame the bytes arrive, which is still before Bevy can start it (it waits for the asset too).
#[test]
fn a_looping_ogg_whose_asset_arrives_later_is_told_when_it_arrives() {
    let mut app = app_for(Mode::Loop);
    let handle = app.world().resource::<Assets<AudioSource>>().reserve_handle();
    app.world_mut().resource_mut::<SirenSounds>().insert("s", handle.clone());
    play(&mut app);
    assert_eq!(duration_of_the_voice(&mut app), Some(None), "nothing to read yet, so nothing is told");

    app.world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .insert(handle.id(), AudioSource { bytes: Arc::from(TONE_OGG) })
        .expect("the reserved handle takes its asset");
    app.update();
    assert!(
        duration_of_the_voice(&mut app).expect("the voice").is_some(),
        "the frame the asset is there, the loop is told its length"
    );
}

/// FLAC and MP3 are bytes Bevy's decoder reads only where `rodio` has those decoders (`mixer`, `semantic`). Without them the crate refuses the play, and there is no voice to be told anything (`tests/undecodable.rs`) — so the rows exist
/// only where the voice can. The same goes for bytes that are not audio: that row was a Bevy-path voice that would have panicked the game in `PostUpdate`, and it is gone.
#[cfg(any(feature = "mixer", feature = "semantic"))]
fn rows_only_a_build_with_those_decoders_can_have() -> Vec<(&'static str, Vec<u8>, Mode)> {
    vec![
        ("a looping FLAC (exact already)", TONE_FLAC.to_vec(), Mode::Loop),
        ("a looping MP3", TONE_MP3.to_vec(), Mode::Loop),
    ]
}

#[cfg(not(any(feature = "mixer", feature = "semantic")))]
fn rows_only_a_build_with_those_decoders_can_have() -> Vec<(&'static str, Vec<u8>, Mode)> {
    Vec::new()
}

/// **What is not told, and why each row matters**: a loop of anything but a stream that declares its length is left exactly as it was, and so is an Ogg that plays once — only a loop exposes the
/// padding, and a one-shot would pay a page walk on every play.
#[test]
fn anything_but_a_looping_ogg_that_declares_its_length_is_told_nothing() {
    let chained = [TONE_441_OGG, TONE_441_OGG].concat(); // two streams back to back: no single length
    let mut cut = TONE_OGG.to_vec();
    cut.truncate(cut.len() - 3); // a last page cut short
    let rows: Vec<(&str, Vec<u8>, Mode)> = [
        ("an Ogg that plays once", TONE_OGG.to_vec(), Mode::Once),
        ("a looping chained Ogg", chained, Mode::Loop),
        ("a looping Ogg cut short", cut, Mode::Loop),
    ]
    .into_iter()
    .chain(rows_only_a_build_with_those_decoders_can_have())
    .collect();
    for (what, bytes, mode) in rows {
        let mut app = playing(&bytes, mode);
        play(&mut app);
        assert_eq!(duration_of_the_voice(&mut app), Some(None), "told nothing: {what}");
    }
}

/// **The arithmetic, against `rodio` itself.** The duration the crate hands Bevy must make `TakeDuration` emit exactly the stream's length — not a sample more, which is the padding it exists to remove, and not
/// one fewer. Bevy builds its decoder and its loop as below (`audio_source.rs:96`, `audio_output.rs:161`); the lengths are `libvorbis`'s own and each file's last granule position (`tests/semantic_decode.rs`).
#[cfg(any(feature = "mixer", feature = "semantic"))]
#[test]
fn the_duration_plays_exactly_the_streams_length_through_rodio_itself() {
    use std::io::Cursor;

    use rodio::Source;

    for (name, bytes, frames, channels) in
        [("tone.ogg", TONE_OGG, 24_000usize, 2usize), ("tone-441.ogg", TONE_441_OGG, 4_416, 2)]
    {
        let mut app = playing(bytes, Mode::Loop);
        play(&mut app);
        let duration = duration_of_the_voice(&mut app).flatten().expect("a looping Ogg is told its length");

        let decoder = || {
            rodio::Decoder::builder()
                .with_byte_len(bytes.len() as u64)
                .with_data(Cursor::new(Arc::<[u8]>::from(bytes)))
                .build()
                .expect("the fixture decodes")
        };
        let (untrimmed, trimmed) = (decoder().count(), decoder().take_duration(duration).count());
        assert_eq!(trimmed, frames * channels, "{name}: exactly the declared length, in samples");
        assert!(
            untrimmed > trimmed,
            "{name}: the decoder returns {untrimmed} samples for a stream that declares {trimmed}"
        );

        // A loop of it recurs with exactly that period: pass two is pass one, so nothing sits between them. **And the loop must produce samples at all.** With the symphonia Vorbis backend (`rodio/vorbis`)
        // a decoder reports a span of length zero until its first sample, and `repeat_infinite` of it is *empty* (RustAudio/rodio#775): Bevy's own loop (`audio_output.rs:161`, `:165`) played nothing, for the
        // whole game, merely because this crate's `mixer` or `semantic` feature was on — and the two assertions below held vacuously for as long as nothing compared a length. `lewton`, which is what
        // Bevy's own `vorbis` feature selects, loops, and is what `Cargo.toml` enables here.
        let mut looped = decoder().take_duration(duration).repeat_infinite();
        let first: Vec<f32> = looped.by_ref().take(trimmed).collect();
        let second: Vec<f32> = looped.by_ref().take(trimmed).collect();
        assert_eq!(
            first.len(),
            trimmed,
            "{name}: the loop produced {} of {trimmed} samples: which Vorbis decoder does `rodio` have here (see `Cargo.toml`)?",
            first.len()
        );
        assert_eq!(first, second, "{name}: the second pass starts where the first ends");
    }
}
