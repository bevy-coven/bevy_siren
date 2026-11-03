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
        let p = std::env::temp_dir().join(format!("bevy_siren_decode_{name}_{}", std::process::id()));
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
fn a_file_at_another_rate_is_decoded_and_then_refused_naming_both_rates() {
    // The commonest case in a real library. Decoding must succeed — the refusal is about conversion, not reading —
    // and the message must name the file's rate and the model's.
    let s = Scratch::new("441");
    let p = s.file("cd.wav", &wav_tone(44_100, 1_000.0, 4410));

    let decoded = decode(&p).expect("44.1 kHz still decodes");
    assert_eq!(decoded.sample_rate, 44_100);

    let err = decode_for_model(&p).expect_err("not converted yet");
    match &err {
        DecodeProblem::Buffer(BufferProblem::ResamplingRequired { from, to }) => {
            assert_eq!(*from, 44_100);
            assert_eq!(*to, MODEL_SAMPLE_RATE);
        }
        other => panic!("expected ResamplingRequired, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(msg.contains("44100") && msg.contains("48000"), "both rates are in the message: {msg}");
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
