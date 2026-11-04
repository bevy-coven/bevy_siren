//! Decoding a real file, and the refusal that follows when it is not at the model's rate.

#![cfg(feature = "semantic")]

use std::path::Path;

use bevy_siren::semantic::MODEL_SAMPLE_RATE;
use bevy_siren::semantic::buffer::BufferProblem;
use bevy_siren::semantic::decode::{DecodeProblem, decode, decode_for_model};

/// A 16-bit mono WAV at `rate`, holding `frames` samples of a `freq` Hz tone.
fn wav_tone(rate: u32, freq: f64, frames: usize) -> Vec<u8> {
    let data_len = (frames * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // mono
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let s = (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin();
        let q = (s * 0.5 * f64::from(i16::MAX)) as i16;
        v.extend_from_slice(&q.to_le_bytes());
    }
    v
}

struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        // One directory per call, whatever the label (see `semantic_indexer.rs`: two tests that shared a label raced for one directory).
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p =
            std::env::temp_dir().join(format!("bevy_siren_decode_{name}_{}_{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch");
        Scratch(p)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = self.0.join(name);
        std::fs::write(&p, bytes).expect("write");
        p
    }
}

#[test]
fn a_wav_decodes_to_its_own_rate_and_channels() {
    let s = Scratch::new("rate");
    let p = s.file("tone.wav", &wav_tone(48_000, 1_000.0, 4800));
    let d = decode(&p).expect("decodes");
    assert_eq!(d.sample_rate, 48_000, "the rate the stream reports");
    assert_eq!(d.channels, 1);
    assert_eq!(d.samples.len(), 4800, "mono, so one sample per frame");
    assert_eq!(d.frames(), 4800);
    assert!((d.seconds() - 0.1).abs() < 1e-6, "4800 frames at 48 kHz is 0.1 s, got {}", d.seconds());
}

#[test]
fn the_decoded_audio_is_the_tone_that_was_written() {
    // A decode that produced noise would still pass a length check. Zero crossings say it is the signal.
    let s = Scratch::new("tone");
    let p = s.file("t.wav", &wav_tone(48_000, 440.0, 24_000));
    let d = decode(&p).expect("decodes");
    let crossings = d.samples.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    // 440 Hz for 0.5 s is 220 cycles, so 220 rising crossings — the first sample is zero and rises, so the count is
    // exact rather than approximate, and a decode that shifted the rate would move it.
    assert_eq!(crossings, 220, "440 Hz for 0.5 s is exactly 220 rising crossings; got {crossings}");
}

#[test]
fn a_file_at_the_models_rate_goes_all_the_way_to_a_model_buffer() {
    let s = Scratch::new("model");
    let p = s.file("ok.wav", &wav_tone(MODEL_SAMPLE_RATE, 1_000.0, 9600));
    let buffer = decode_for_model(&p).expect("48 kHz is a model buffer");
    assert_eq!(buffer.len(), 9600);
    assert!(buffer.iter().all(|x| x.is_finite()));
}

#[test]
fn a_file_at_another_rate_is_converted_not_refused() {
    // **The behaviour #12 closed on.** 44.1 kHz is the commonest rate a game ships; it must reach the model, at the
    // model's rate and the right duration, rather than being turned away.
    let s = Scratch::new("441");
    let rate = 44_100u32;
    let seconds = 0.5;
    let frames = (f64::from(rate) * seconds) as usize;
    let p = s.file("cd.wav", &wav_tone(rate, 1_000.0, frames));

    let decoded = decode(&p).expect("decodes");
    assert_eq!(decoded.sample_rate, 44_100, "the file's own rate");

    let buffer = decode_for_model(&p).expect("a 44.1 kHz file is converted, not refused");
    let expected = ((frames as f64) * (f64::from(MODEL_SAMPLE_RATE)) / f64::from(rate)).round() as usize;
    assert_eq!(buffer.len(), expected, "0.5 s stays 0.5 s: {frames} at 44.1 kHz is {expected} at 48 kHz");
    assert!(buffer.iter().all(|x| x.is_finite()), "no NaN in the converted buffer");

    // And it is still the tone: 1 kHz for 0.5 s is ~500 rising crossings.
    let crossings = buffer.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
    assert!((497..=503).contains(&crossings), "the tone survives the conversion; got {crossings}");
}

#[test]
fn the_low_level_refusal_still_exists_for_a_caller_that_skips_the_conversion() {
    // `buffer::model_buffer` is public and still refuses a rate it cannot convert. That is the low-level contract;
    // `decode_for_model` is the path that does the conversion first.
    let err = bevy_siren::semantic::buffer::model_buffer(&[0.1f32; 100], 1, 44_100)
        .expect_err("model_buffer itself still refuses");
    assert!(matches!(err, BufferProblem::ResamplingRequired { .. }), "got {err:?}");
}

#[test]
fn a_missing_file_says_which_file() {
    let err = decode(Path::new("/nonexistent/nope.wav")).expect_err("cannot open");
    match &err {
        DecodeProblem::Open { path, .. } => assert_eq!(path, "/nonexistent/nope.wav"),
        other => panic!("expected Open, got {other:?}"),
    }
    assert!(err.to_string().contains("/nonexistent/nope.wav"), "{err}");
}

#[test]
fn a_file_that_is_not_audio_is_refused_rather_than_panicking() {
    let s = Scratch::new("junk");
    let p = s.file("notes.wav", b"this is not a wav file, whatever the extension says");
    let err = decode(&p).expect_err("not audio");
    assert!(matches!(err, DecodeProblem::Undecodable { .. }), "got {err:?}");
    assert!(err.to_string().contains("notes.wav"), "the message names the file: {err}");
}

// ---------------------------------------------------------------------------------------------------------------
// Every format the scan calls audio, decoded from a real file of that format.
// ---------------------------------------------------------------------------------------------------------------

// **The fixtures.** `tests/fixtures/tone*.{flac,mp3,ogg}` are a sine wave, made once with `ffmpeg 9.0.2`:
//
//     SRC='sine=frequency=440:sample_rate=48000:duration=0.5'    # lavfi's sine is at 1/8 of full scale: a peak of 0.125
//     ffmpeg -f lavfi -i "$SRC" -ac 1 -c:a flac                tone.flac   # lossless
//     ffmpeg -f lavfi -i "$SRC" -ac 1 -c:a libmp3lame -b:a 64k tone.mp3
//     ffmpeg -f lavfi -i "$SRC" -ac 2 -c:a vorbis -strict -2   tone.ogg    # the native encoder does one or two channels
//     # and from `sine=frequency=1000:sample_rate=44100:duration=0.1`, as 44.1 kHz stereo: tone-441.flac, tone-441.ogg
//
// Real files from an independent encoder, not bytes this crate wrote: a decoder tested only on its own author's encoder proves
// that the pair agree, not that either is right.
const TONE_FLAC: &[u8] = include_bytes!("fixtures/tone.flac");
const TONE_MP3: &[u8] = include_bytes!("fixtures/tone.mp3");
const TONE_OGG: &[u8] = include_bytes!("fixtures/tone.ogg");

/// The first channel of an interleaved decode.
fn first_channel(d: &bevy_siren::semantic::decode::Decoded) -> Vec<f32> {
    d.samples.iter().step_by(usize::from(d.channels)).copied().collect()
}

fn rising_crossings(x: &[f32]) -> usize {
    x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
}

/// **Cycles, counted through a dead band.** A lossy codec's quantisation noise near a zero crossing makes the signal chatter across
/// zero several times, so a plain crossing count read 231 for a 220-cycle Vorbis tone — not a decode at the wrong pitch (that moves
/// the count by a factor, not by 5 %), but noise on a correct one. A cycle here is a rise through `+band` after a fall through
/// `-band`; the band is a quarter of the tone's 0.125 peak, far above codec noise and far below the tone.
fn cycles(x: &[f32]) -> usize {
    const BAND: f32 = 0.03;
    let (mut armed, mut count) = (false, 0usize);
    for v in x {
        if *v < -BAND {
            armed = true;
        } else if *v > BAND && armed {
            count += 1;
            armed = false;
        }
    }
    count
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0f32, |m, v| m.max(v.abs()))
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt()
}

/// What a sine of peak `amplitude` and 440 Hz measures over 0.5 s: 220 cycles (the dead-band count arms only after the first fall,
/// so a clean decode reads 219 or 220), that peak, and an RMS of `amplitude / √2` — each within 12 %, which a gain error (a missed
/// 16-bit scale, a doubled channel) is far outside of and a lossy codec's rounding is well inside.
fn assert_is_the_tone(name: &str, x: &[f32], amplitude: f32, cycle_count: std::ops::RangeInclusive<usize>) {
    assert!(cycle_count.contains(&cycles(x)), "{name}: 440 Hz for 0.5 s is 220 cycles; got {}", cycles(x));
    let near = |got: f32, want: f32| (got - want).abs() <= 0.12 * want;
    assert!(
        near(peak(x), amplitude),
        "{name}: a sine of amplitude {amplitude} peaks near {amplitude}, not {}",
        peak(x)
    );
    let want = amplitude / std::f32::consts::SQRT_2;
    assert!(near(rms(x), want), "{name}: its RMS is {want}, not {}", rms(x));
}

/// **FLAC is lossless, so it decodes to exactly the tone that was encoded** — the one format where "is it right" has an exact
/// answer: the samples are the 16-bit values `ffmpeg` generated, and the only distance from the ideal sine is that rounding.
#[test]
fn a_flac_decodes_to_exactly_the_tone_that_was_encoded() {
    let s = Scratch::new("flac");
    let d = decode(&s.file("tone.flac", TONE_FLAC)).expect("a FLAC file decodes");
    assert_eq!((d.sample_rate, d.channels), (48_000, 1), "the rate and channels the stream reports");
    assert_eq!(d.frames(), 24_000, "lossless and gapless: 0.5 s at 48 kHz is exactly 24000 frames");
    assert_is_the_tone("flac", &d.samples, 0.125, 219..=220);
    assert_eq!(
        rising_crossings(&d.samples),
        220,
        "lossless: no noise, so the plain crossing count is exact too"
    );

    let ideal = |i: usize| 0.125 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / 48_000.0).sin();
    let squared: f64 = d.samples.iter().enumerate().map(|(i, v)| (f64::from(*v) - ideal(i)).powi(2)).sum();
    let error = (squared / d.samples.len() as f64).sqrt();
    assert!(
        error < 1e-4,
        "a lossless decode differs from the ideal tone only by 16-bit rounding (~1e-5); rms error {error}"
    );
}

/// **MP3 is lossy and has an encoder delay**, so it is judged by what a listener would: the right rate, about the right length,
/// and the right pitch and level. An exact-sample comparison would pin the decoder's gapless handling instead of its correctness.
#[test]
fn an_mp3_decodes_to_the_tone_that_was_encoded() {
    let s = Scratch::new("mp3");
    let d = decode(&s.file("tone.mp3", TONE_MP3)).expect("an MP3 file decodes");
    assert_eq!((d.sample_rate, d.channels), (48_000, 1), "the rate and channels the stream reports");
    assert!(
        (22_800..=25_200).contains(&d.frames()),
        "0.5 s is 24000 frames, give or take the encoder's padding (one MP3 frame is 1152); got {}",
        d.frames()
    );
    assert_is_the_tone("mp3", &d.samples, 0.125, 218..=221);
}

/// **Ogg Vorbis** — the format the crate's own docs call "Bevy's default audio format" — had no test at all.
#[test]
fn an_ogg_vorbis_decodes_to_the_tone_that_was_encoded() {
    let s = Scratch::new("ogg");
    let d = decode(&s.file("tone.ogg", TONE_OGG)).expect("an Ogg Vorbis file decodes");
    assert_eq!((d.sample_rate, d.channels), (48_000, 2), "the rate and channels the stream reports");
    assert!(
        (22_800..=25_200).contains(&d.frames()),
        "0.5 s is 24000 frames, give or take padding; got {}",
        d.frames()
    );
    let left = first_channel(&d);
    // `ffmpeg -ac 2` upmixes the mono tone with its default matrix, which puts it on each channel at -3 dB: a peak of 0.125/√2.
    assert_is_the_tone("ogg", &left, 0.125 / std::f32::consts::SQRT_2, 218..=221);
    // The encoder was given the same tone on both channels, so the decode must give two that agree.
    let right: Vec<f32> = d.samples.iter().skip(1).step_by(2).copied().collect();
    let worst = left.iter().zip(&right).map(|(l, r)| (l - r).abs()).fold(0.0f32, f32::max);
    assert!(worst < 0.02, "both channels carry the same tone; they differ by up to {worst}");
}

/// **An Ogg Vorbis stream ends on a page that says how many frames it has, and a decoder must stop there.** The last packet decodes to a
/// whole block — up to half a long block more than the stream holds — and the Vorbis spec says to discard the excess (Vorbis I, A.2).
/// `symphonia` 0.5 does not: on 209 real Ogg effects, compared with `libvorbis`, **59 came out up to 975 frames long** (20 ms at 48 kHz)
/// and none short; on the RPG pack's effects (0.27 – 1.5 s) that tail alone moved a similarity by up to 0.085 and changed the top
/// three of all 12 descriptions tried, and cut to the declared length the worst difference was 0.004 and the rank correlation never
/// fell below 0.998 (`docs/loop/runs/2026-11-03-112/evidence/after/formats-parity-ogg-vs-libvorbis.txt`, `ogg-tail-vs-clamp.txt`).
///
/// The lengths below are **`libvorbis`'s own** (`soundfile`, which is libsndfile) **and the granule position of each file's last page** —
/// two independent statements of one number — and not what this decoder happened to return, which was 24 576 and 5 120.
#[test]
fn an_ogg_is_cut_to_the_length_its_last_page_declares() {
    let s = Scratch::new("ogg-trim");
    let long = decode(&s.file("tone.ogg", TONE_OGG)).expect("decodes");
    assert_eq!(
        (long.sample_rate, long.frames()),
        (48_000, 24_000),
        "0.5 s at 48 kHz exactly — not the 24 576 frames the last block holds"
    );
    let short = decode(&s.file("tone-441.ogg", include_bytes!("fixtures/tone-441.ogg"))).expect("decodes");
    assert_eq!(
        (short.sample_rate, short.frames()),
        (44_100, 4_416),
        "the stream declares 4416 frames (the encoder rounds 0.1 s up to its block alignment), not the 5120 the last block holds"
    );
}

/// **A chained Ogg has no single length, so nothing is cut.** Two streams back to back (`cat a.ogg a.ogg`) end on the *second* stream's
/// last page, whose granule position is that stream's own frame count; cutting the whole file to it would throw away real audio. The
/// trim reads a length only from a file that is one stream, and a decoder that refuses a chained file has cut nothing either.
#[test]
fn a_chained_ogg_is_not_cut_to_its_last_streams_length() {
    let s = Scratch::new("ogg-chained");
    let one: &[u8] = include_bytes!("fixtures/tone-441.ogg");
    let chained = [one, one].concat();
    if let Ok(d) = decode(&s.file("chained.ogg", &chained)) {
        assert!(d.frames() > 4_416, "the first stream's audio is still there: {} frames", d.frames());
    }
}

/// **The scan's list and the decoder's features are two statements of one fact, and they had drifted apart.** `mp3` and `flac` were
/// in `AUDIO_EXTENSIONS` — documented as *"the formats the crate can decode"* — and in `indexer`'s own error message (*"a library of
/// `.mp3` files is indexed"*), and `rodio` was never given the features to decode either: every such file was scanned as audio and
/// then refused as undecodable. Nothing tested it, because no test had a file that was not a WAV.
///
/// This takes each extension on the list, requires a fixture for it, and decodes that fixture **through the model path** — so a new
/// entry on the list without a decoder (or without a fixture) fails here, in this crate's own words.
#[test]
fn every_extension_the_scan_calls_audio_has_a_decoder() {
    use bevy_siren::library::AUDIO_EXTENSIONS;

    let table: [(&str, Vec<u8>); 5] = [
        ("wav", wav_tone(48_000, 440.0, 24_000)),
        ("ogg", TONE_OGG.to_vec()),
        ("oga", TONE_OGG.to_vec()), // Ogg Vorbis under its other extension: the container, not the name, is what is read
        ("flac", TONE_FLAC.to_vec()),
        ("mp3", TONE_MP3.to_vec()),
    ];
    let mut listed: Vec<&str> = AUDIO_EXTENSIONS.to_vec();
    let mut tested: Vec<&str> = table.iter().map(|(extension, _)| *extension).collect();
    listed.sort_unstable();
    tested.sort_unstable();
    assert_eq!(
        listed, tested,
        "the scan's list of audio and this test's table of fixtures must be the same set"
    );

    let s = Scratch::new("every");
    for (extension, bytes) in &table {
        let path = s.file(&format!("tone.{extension}"), bytes);
        let buffer = decode_for_model(&path).unwrap_or_else(|e| {
            panic!("`.{extension}` is on the scan's list of audio and does not decode: {e}")
        });
        assert!(
            buffer.len() > 20_000 && buffer.iter().all(|x| x.is_finite()),
            "`.{extension}` gave {} samples",
            buffer.len()
        );
    }
}
