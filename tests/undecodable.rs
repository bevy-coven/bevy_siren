//! **Bytes that load and do not decode are refused, with the cause — not handed to a decoder that unwraps them.**
//!
//! Bevy's `AudioSource::decoder` is `rodio::Decoder::builder()…build().unwrap()` (`bevy_audio-0.20.1/src/audio_source.rs:96-101`) and `play_queued_audio_system` calls it in `PostUpdate` for every voice that has an `AudioPlayer`
//! and no sink. A file that loads — the asset loader returns the bytes and stops — and is not audio this build can decode (a Git LFS pointer, an empty file, a cut one, an Ogg *Opus* file, an ADPCM WAV) therefore **panics
//! the game in the frame the play was answered `Played`**, on the default build, and the message names no file (`docs/loop/runs/2026-11-04-117/evidence/before/`). The crate's own mixer has always refused such bytes
//! (`start_pending`); a headless test never saw the other path, because nothing in a headless app runs Bevy's decoder.
//!
//! These tests need no device. What they hold is **the crate's answer before Bevy's decode could run**: a refusal, a freed budget slot, and no `AudioPlayer` left for `PostUpdate` to be handed. And the universal property
//! that makes the check safe — *the crate refuses exactly the bytes `AudioSource::decoder()` panics on*, in every feature set.
//!
//! **Bevy's path is forced in every build, by removing the `Output`** (a `mixer` build on a machine with a device inserts one, and then the crate's own graph decides, in `start_pending`).
//!
//! Fixtures `opus.ogg` and `adpcm.wav` are 0.1 s of a 440 Hz sine, made by an encoder that is not this crate's:
//! `ffmpeg -f lavfi -i "sine=frequency=440:duration=0.1" -ac 1 -c:a libopus tests/fixtures/opus.ogg` and
//! `ffmpeg -f lavfi -i "sine=frequency=440:duration=0.1" -ac 1 -ar 22050 -c:a adpcm_ms tests/fixtures/adpcm.wav`.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use bevy::audio::{AudioPlayer, AudioSource, Decodable};
use bevy::prelude::*;
use bevy_siren::{
    Cue, CueTable, Output, Play, Refusal, SirenLog, SirenMix, SirenSettings, SirenSounds, SoundSlot, Voice,
};

use crate::helpers::{app, played, refused, sounds, voices};

const TONE_OGG: &[u8] = include_bytes!("fixtures/tone.ogg");
const TONE_441_OGG: &[u8] = include_bytes!("fixtures/tone-441.ogg");
const TONE_FLAC: &[u8] = include_bytes!("fixtures/tone.flac");
const TONE_MP3: &[u8] = include_bytes!("fixtures/tone.mp3");
const OPUS_OGG: &[u8] = include_bytes!("fixtures/opus.ogg");
const ADPCM_WAV: &[u8] = include_bytes!("fixtures/adpcm.wav");
/// What a repository that stores its audio in Git LFS holds where the audio should be (`version`, `oid`, `size`: the pointer file's three lines).
const LFS_POINTER: &[u8] = b"version https://git-lfs.github.com/spec/v1\noid sha256:4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393\nsize 20838\n";

/// A headless app, on Bevy's path, whose cue `"click"` plays the sound `"s"` — which points at nothing yet.
fn app_for_click() -> App {
    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("s")], ..Cue::new("click") }]);
    let mut app = app(table, SirenSettings::default(), sounds(&["s"]), false);
    app.world_mut().remove_resource::<Output>();
    app
}

/// An app whose sound `"s"` is `bytes`, **already loaded**, and the handle, so a test can edit the file.
fn loaded(bytes: &[u8]) -> (App, Handle<AudioSource>) {
    let mut app = app_for_click();
    let handle =
        app.world_mut().resource_mut::<Assets<AudioSource>>().add(AudioSource { bytes: Arc::from(bytes) });
    app.world_mut().resource_mut::<SirenSounds>().insert("s", handle.clone());
    (app, handle)
}

fn click(app: &mut App) {
    app.world_mut().write_message(Play::cue("click"));
    app.update();
}

fn lines(app: &App) -> Vec<String> {
    app.world().resource::<SirenLog>().iter().map(|entry| entry.line()).collect()
}

/// How many voices Bevy would be handed in `PostUpdate`.
fn players(app: &mut App) -> usize {
    app.world_mut().query::<&AudioPlayer>().iter(app.world()).count()
}

/// Bytes that are not audio this build can decode, each with the **hint** the log should add (the decoder's own words are the same for all of them).
struct Row {
    what: &'static str,
    bytes: Vec<u8>,
    hint: Option<&'static str>,
}

fn undecodable_rows() -> Vec<Row> {
    vec![
        Row { what: "a Git LFS pointer", bytes: LFS_POINTER.to_vec(), hint: Some("Git LFS pointer") },
        Row { what: "a file with no bytes", bytes: Vec::new(), hint: Some("no bytes") },
        Row { what: "the first 40 bytes of a real Ogg", bytes: TONE_OGG[..40].to_vec(), hint: None },
        Row {
            what: "a WAV header that lies",
            bytes: b"RIFF\x00\x00\x00\x00WAVEthis is not a fmt chunk".to_vec(),
            hint: None,
        },
        Row { what: "an Ogg Opus file", bytes: OPUS_OGG.to_vec(), hint: Some("Opus") },
        Row { what: "an ADPCM WAV", bytes: ADPCM_WAV.to_vec(), hint: None },
    ]
}

/// **A play of bytes the decoder refuses is refused where it is asked — naming the file and the cause — and plays nothing.** The unchanged tree answered `Played` for every row and the game panicked in that frame.
#[test]
fn a_play_of_bytes_the_decoder_refuses_is_refused_with_the_cause() {
    for row in undecodable_rows() {
        let (mut app, _) = loaded(&row.bytes);
        click(&mut app);

        assert!(
            played(&mut app).is_empty(),
            "{}: nothing plays on a refusal, so nothing is announced as played",
            row.what
        );
        let answers = refused(&mut app);
        assert_eq!(answers.len(), 1, "{}: one play, one answer: {answers:?}", row.what);
        assert_eq!(
            answers[0].reason,
            Refusal::MissingSound { sound: "s".to_owned() },
            "{}: the key to fix, not the cue",
            row.what
        );
        assert_eq!(voices(&mut app), 0, "{}: no voice, so no AudioPlayer for Bevy to unwrap", row.what);
        assert_eq!(players(&mut app), 0, "{}: nothing for PostUpdate to be handed", row.what);
        assert_eq!(
            app.world().resource::<SirenMix>().voices_on("Effects"),
            0,
            "{}: and nothing is counted against the budget",
            row.what
        );

        let log = lines(&app);
        let line = log
            .iter()
            .find(|l| l.contains("refused"))
            .unwrap_or_else(|| panic!("{}: the log says it refused: {log:?}", row.what));
        assert!(
            line.contains("`s`") && line.contains("cannot be decoded"),
            "{}: the line names the sound and says the bytes cannot be decoded: {line}",
            row.what
        );
        let cause = line.split("cannot be decoded:").nth(1).map(str::trim).unwrap_or_default();
        assert!(!cause.is_empty(), "{}: and carries the decoder's own words: {line}", row.what);
        if let Some(hint) = row.hint {
            assert!(line.contains(hint), "{}: and says what the file is: {line}", row.what);
        }
        assert!(
            !log.iter().any(|l| l.contains(" played ")),
            "{}: and never says it played: {log:?}",
            row.what
        );
    }
}

/// **The check is Bevy's own, so it cannot disagree with it.** For every row — the ones that are not audio *and* every good fixture — the crate refuses a play **exactly when `AudioSource::decoder()` panics**.
/// A property of the build and not a list of formats: an MP3 or a FLAC is refused in the default build, where `rodio` has no decoder for them, and played where it has one (`mixer`, `semantic`); a file a decoder
/// reads is never refused, and a file it would panic on is never played.
#[test]
fn the_crate_refuses_exactly_the_bytes_bevys_own_decoder_panics_on() {
    fn bevy_would_panic(bytes: &[u8]) -> bool {
        let source = AudioSource { bytes: Arc::from(bytes) };
        catch_unwind(AssertUnwindSafe(|| {
            let _ = source.decoder();
        }))
        .is_err()
    }

    let mut rows: Vec<(String, Vec<u8>)> =
        undecodable_rows().into_iter().map(|row| (row.what.to_owned(), row.bytes)).collect();
    for (what, bytes) in [
        ("a real Ogg Vorbis", TONE_OGG),
        ("another real Ogg Vorbis", TONE_441_OGG),
        ("a real FLAC", TONE_FLAC),
        ("a real MP3", TONE_MP3),
    ] {
        rows.push((what.to_owned(), bytes.to_vec()));
    }

    let (mut refused_rows, mut played_rows) = (0, 0);
    for (what, bytes) in rows {
        let would_panic = bevy_would_panic(&bytes);
        let (mut app, _) = loaded(&bytes);
        click(&mut app);
        let (answered_played, answered_refused) = (played(&mut app).len(), refused(&mut app).len());
        assert_eq!(
            answered_refused == 1,
            would_panic,
            "{what}: refused exactly when Bevy's decoder panics (it would panic: {would_panic}; played {answered_played}, refused {answered_refused})"
        );
        if would_panic {
            refused_rows += 1;
            assert_eq!(answered_played, 0, "{what}: refused, so not played");
            assert_eq!(voices(&mut app), 0, "{what}: and no voice");
        } else {
            played_rows += 1;
            assert_eq!(answered_played, 1, "{what}: a sound Bevy can decode is played");
            assert_eq!(voices(&mut app), 1, "{what}: and sounds as a voice");
        }
    }
    assert!(refused_rows >= 6, "the six rows that are not audio were all refused ({refused_rows})");
    assert!(played_rows >= 2, "and the two Oggs were played, in every feature set ({played_rows})");
}

/// **Bytes that arrive after the play are refused in the frame they arrive — before Bevy's `PostUpdate` can decode them.** The play could not know (the asset was still loading), so `Played` was written and the voice
/// waits for its bytes, as it always did; the crate's systems run in `Update`, Bevy's decoder in `PostUpdate`, so a voice whose bytes turn out undecodable is refused and despawned in between. Bytes the decoder
/// reads are left to wait for Bevy exactly as before.
#[test]
fn bytes_that_arrive_after_the_play_are_refused_before_bevy_can_start_the_voice() {
    for row in undecodable_rows() {
        let mut app = app_for_click();
        let handle = app.world().resource::<Assets<AudioSource>>().reserve_handle();
        app.world_mut().resource_mut::<SirenSounds>().insert("s", handle.clone());
        click(&mut app);
        assert_eq!(played(&mut app).len(), 1, "{}: a play whose bytes are still loading is played", row.what);
        assert_eq!(voices(&mut app), 1, "{}: and waits for them", row.what);
        assert!(refused(&mut app).is_empty(), "{}: and is not refused: nothing is known", row.what);

        app.world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .insert(handle.id(), AudioSource { bytes: Arc::from(row.bytes.as_slice()) })
            .expect("the reserved handle takes its asset");
        app.update();

        let answers = refused(&mut app);
        assert_eq!(answers.len(), 1, "{}: refused in the frame the bytes arrive: {answers:?}", row.what);
        assert_eq!(answers[0].reason, Refusal::MissingSound { sound: "s".to_owned() }, "{}", row.what);
        assert_eq!(voices(&mut app), 0, "{}: the voice is gone", row.what);
        assert_eq!(
            players(&mut app),
            0,
            "{}: and so is its AudioPlayer, which is what Bevy would have decoded",
            row.what
        );
        assert_eq!(
            app.world().resource::<SirenMix>().voices_on("Effects"),
            0,
            "{}: and its slot is free",
            row.what
        );
        let log = lines(&app);
        let (said_played, said_refused) = (
            log.iter().position(|l| l.contains(" played ")),
            log.iter().position(|l| l.contains("refused") && l.contains("cannot be decoded")),
        );
        assert!(
            said_played.is_some() && said_refused > said_played,
            "{}: played, then refused with the cause: {log:?}",
            row.what
        );
    }
}

/// **The control: bytes the decoder reads are never refused, whenever they arrive.** A decodable sound that is already loaded is played; one that arrives after the play keeps its voice, waiting for Bevy as it always did.
#[test]
fn a_sound_the_decoder_reads_is_played_whether_its_bytes_are_there_or_arrive_later() {
    for (what, bytes) in [("a real Ogg", TONE_OGG), ("another real Ogg", TONE_441_OGG)] {
        let (mut app, _) = loaded(bytes);
        click(&mut app);
        assert_eq!(played(&mut app).len(), 1, "{what}: played");
        assert!(refused(&mut app).is_empty(), "{what}: and not refused");
        assert_eq!(voices(&mut app), 1, "{what}: a voice for Bevy to start");
        assert_eq!(players(&mut app), 1, "{what}: with its AudioPlayer");
    }

    let mut app = app_for_click();
    let handle = app.world().resource::<Assets<AudioSource>>().reserve_handle();
    app.world_mut().resource_mut::<SirenSounds>().insert("s", handle.clone());
    click(&mut app);
    app.world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .insert(handle.id(), AudioSource { bytes: Arc::from(TONE_OGG) })
        .expect("the reserved handle takes its asset");
    for _ in 0..3 {
        app.update();
    }
    assert!(refused(&mut app).is_empty(), "a decodable sound arriving late is not refused");
    assert_eq!(voices(&mut app), 1, "and its voice waits for Bevy");
}

/// **An edit is asked again.** The verdict belongs to the bytes, not to the handle: a file fixed on disk plays on the next play, and one broken on disk is refused on the next — including a corruption that keeps the
/// file's length *and* its first bytes (one flipped byte inside the second Ogg page, which no longer checks), which only the buffer's identity tells apart.
#[test]
fn an_edit_to_the_file_is_asked_again() {
    let replace = |app: &mut App, handle: &Handle<AudioSource>, bytes: &[u8]| {
        app.world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .insert(handle.id(), AudioSource { bytes: Arc::from(bytes) })
            .expect("the asset is replaced");
    };

    // Bad, then fixed.
    let (mut app, handle) = loaded(LFS_POINTER);
    click(&mut app);
    assert_eq!(refused(&mut app).len(), 1, "the pointer is refused");
    replace(&mut app, &handle, TONE_OGG);
    click(&mut app);
    assert_eq!(played(&mut app).len(), 1, "the fixed file plays on the next play");
    assert!(refused(&mut app).is_empty());

    // Good, then broken in a way that keeps its length and its head.
    let mut broken = TONE_OGG.to_vec();
    broken[300] ^= 0xFF;
    assert_eq!(
        (broken.len(), &broken[..16]),
        (TONE_OGG.len(), &TONE_OGG[..16]),
        "the premise: same length, same head"
    );
    let (mut app, handle) = loaded(TONE_OGG);
    click(&mut app);
    assert_eq!(played(&mut app).len(), 1, "the good file plays");
    // Headless, nothing starts that voice (there is no device for Bevy to open), so it would itself be refused in flight when its bytes change. Take it out of the way: the next play is what is asked about.
    let ids: Vec<Entity> =
        app.world_mut().query_filtered::<Entity, With<Voice>>().iter(app.world()).collect();
    for id in ids {
        app.world_mut().despawn(id);
    }
    replace(&mut app, &handle, &broken);
    click(&mut app);
    let answers = refused(&mut app);
    assert_eq!(
        answers.len(),
        1,
        "the broken one is refused on the next play, though nothing about its length or head changed: {answers:?}"
    );
    assert!(played(&mut app).is_empty());
}
