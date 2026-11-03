//! The buffer arithmetic that feeds a model, tested with no decoder, no device and no `App`.
//!
//! Every number here is a ratio a person can check by hand: a rate ratio, a length, a peak.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::MODEL_SAMPLE_RATE;
use bevy_siren::semantic::buffer::{BufferProblem, downmix_to_mono, model_buffer, peak};

#[test]
fn a_stereo_file_folds_down_to_its_average_not_its_sum() {
    // L = 0.8, R = -0.2 -> mean 0.3. A sum would say 0.6 and put every stereo asset 6 dB above every mono one.
    let stereo = [0.8f32, -0.2, 0.4, 0.4];
    let mono = downmix_to_mono(&stereo, 2).expect("stereo folds down");
    assert_eq!(mono.len(), 2, "two frames from four samples");
    assert!((mono[0] - 0.3).abs() < 1e-6, "mean of 0.8 and -0.2 is 0.3, got {}", mono[0]);
    assert!((mono[1] - 0.4).abs() < 1e-6, "mean of 0.4 and 0.4 is 0.4, got {}", mono[1]);
}

#[test]
fn a_zero_channel_file_is_refused_rather_than_dividing_by_zero() {
    assert_eq!(downmix_to_mono(&[1.0, 2.0], 0), Err(BufferProblem::NoChannels));
}

#[test]
fn a_non_finite_sample_becomes_silence_rather_than_poisoning_the_buffer() {
    let mono = downmix_to_mono(&[f32::NAN, 0.5, f32::INFINITY, 0.5], 1).expect("mono");
    assert!(mono.iter().all(|s| s.is_finite()), "no NaN or infinity survives: {mono:?}");
    assert_eq!(mono[0], 0.0, "NaN becomes silence");
    assert_eq!(mono[2], 0.0, "infinity becomes silence");
}

#[test]
fn a_file_at_the_models_rate_goes_straight_through() {
    let mono: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
    let out = model_buffer(&mono, 1, MODEL_SAMPLE_RATE).expect("48 kHz mono is already a model buffer");
    assert_eq!(out, mono, "no arithmetic when no conversion is needed");
}

#[test]
fn a_file_at_any_other_rate_is_refused_naming_both_rates() {
    // 44.1 kHz is the commonest rate a game ships, so this is the case a developer will actually hit. It must say
    // what it found, what it needs, and where the decision lives — not blame the file.
    let err = model_buffer(&[0.1f32; 100], 1, 44_100).expect_err("44.1 kHz is not converted yet");
    assert_eq!(err, BufferProblem::ResamplingRequired { from: 44_100, to: MODEL_SAMPLE_RATE });
    let text = err.to_string();
    assert!(text.contains("44100"), "the message names the file's rate: {text}");
    assert!(text.contains("48000"), "and the rate it needs: {text}");
    assert!(text.contains("needs-decision"), "and where the decision lives: {text}");
}

#[test]
fn a_refusal_is_never_a_silently_padded_buffer() {
    // The failure mode this guards: a conversion that "works" on an aliased signal. A refusal returns no buffer at
    // all, so nothing downstream can mistake one for audio.
    assert!(model_buffer(&[0.5f32; 441], 1, 22_050).is_err());
}

#[test]
fn a_zero_rate_is_refused_at_both_ends() {
    assert_eq!(model_buffer(&[0.1], 1, 0), Err(BufferProblem::ZeroSampleRate));
}

#[test]
fn an_empty_decode_says_so() {
    assert_eq!(model_buffer(&[], 1, 48_000), Err(BufferProblem::Empty));
}

#[test]
fn a_model_buffer_is_at_the_models_rate_and_mono() {
    let stereo: Vec<f32> = (0..4410)
        .flat_map(|i| {
            let v = (i as f32 * 0.01).sin();
            [v, v]
        })
        .collect();
    let buffer = model_buffer(&stereo, 2, MODEL_SAMPLE_RATE).expect("model buffer");
    assert_eq!(buffer.len(), 4410, "one mono sample per stereo frame, at a rate already accepted");
    assert!(buffer.iter().all(|s| s.is_finite()));
}

#[test]
fn peak_reads_the_largest_absolute_sample_and_ignores_non_finite() {
    assert_eq!(peak(&[0.1, -0.7, 0.3]), 0.7);
    assert_eq!(peak(&[f32::NAN, 0.2]), 0.2, "a NaN does not mask a real peak");
    assert_eq!(peak(&[]), 0.0);
}
