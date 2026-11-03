//! The conversion: the length the ratio promises, the tone intact, and the flush that was missing.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::MODEL_SAMPLE_RATE;
use bevy_siren::semantic::buffer::BufferProblem;
use bevy_siren::semantic::resample::{convert, to_model_rate};

fn tone(freq: f64, rate: u32, seconds: f64) -> Vec<f32> {
    let n = (f64::from(rate) * seconds).round() as usize;
    (0..n).map(|i| (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin() as f32).collect()
}

/// Rising zero crossings: one per cycle, so this reads back the tone's frequency.
fn crossings(signal: &[f32]) -> usize {
    signal.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count()
}

/// Energy at one frequency, as a single-bin DFT magnitude.
fn energy_at(signal: &[f32], hz: f64, rate: u32) -> f64 {
    let w = 2.0 * std::f64::consts::PI * hz / f64::from(rate);
    let (mut re, mut im) = (0.0f64, 0.0f64);
    for (i, &s) in signal.iter().enumerate() {
        let a = w * i as f64;
        re += f64::from(s) * a.cos();
        im -= f64::from(s) * a.sin();
    }
    (re * re + im * im).sqrt() / signal.len() as f64
}

#[test]
fn one_second_at_44_1k_becomes_exactly_48000_samples() {
    // **The number this issue closes on.** The first probe lost 480 samples because the tail was not flushed;
    // 1.000 s must stay 1.000 s, so 48000 is the contract and 47520 would be the bug.
    let input = tone(1_000.0, 44_100, 1.0);
    assert_eq!(input.len(), 44_100, "the input is exactly one second");
    let out = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    assert_eq!(out.len(), 48_000, "one second at 48 kHz is 48000 samples, not 47520");
}

#[test]
fn the_tone_keeps_its_frequency_through_the_conversion() {
    // A resampler that dropped samples would fold a 1 kHz tone to something else; counting cycles catches that.
    let input = tone(1_000.0, 44_100, 1.0);
    let out = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    let c = crossings(&out);
    assert!((998..=1002).contains(&c), "1 kHz for 1 s is ~1000 cycles at 48 kHz; got {c}");
}

#[test]
fn the_tone_keeps_its_energy_within_the_measured_band() {
    // The probe measured 0.9763 for a 1 kHz tone through `FftFixedInOut`. This pins the same figure, so a swap to a
    // cheaper converter that attenuates or aliases fails here rather than in an embedding.
    let input = tone(1_000.0, 44_100, 0.5);
    let out = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    let e_in = energy_at(&input, 1_000.0, 44_100);
    let e_out = energy_at(&out, 1_000.0, MODEL_SAMPLE_RATE);
    let ratio = e_out / e_in;
    assert!(
        (0.95..=1.02).contains(&ratio),
        "a 1 kHz tone keeps ~0.9763 of its energy (measured); got {ratio:.4}"
    );
}

#[test]
fn a_high_tone_survives_rather_than_aliasing() {
    // 15 kHz is well inside 44.1 kHz's band, so it must come out intact. If the conversion folded it, energy would
    // appear at 9.1 kHz (the image) and not at 15 kHz.
    let input = tone(15_000.0, 44_100, 0.5);
    let out = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    let at_15k = energy_at(&out, 15_000.0, MODEL_SAMPLE_RATE);
    let at_9k = energy_at(&out, 9_100.0, MODEL_SAMPLE_RATE);
    assert!(at_15k > 0.4, "the 15 kHz tone survives: {at_15k:.4}");
    assert!(at_15k > at_9k * 10.0, "and nothing folded to 9.1 kHz: {at_15k:.4} vs {at_9k:.4}");
}

#[test]
fn a_length_that_is_not_a_multiple_of_the_chunk_keeps_its_duration() {
    // The flush case, with a length that cannot divide evenly into 1024.
    for n in [1, 7, 1000, 1023, 1025, 4411] {
        let input = tone(440.0, 44_100, 0.0);
        let mut v = input;
        v.resize(n, 0.1);
        let out = convert(&v, 44_100, MODEL_SAMPLE_RATE).expect("converts");
        let expected = ((n as f64) * (MODEL_SAMPLE_RATE as f64) / 44_100.0).round() as usize;
        assert_eq!(out.len(), expected, "{n} samples must become {expected}, got {}", out.len());
    }
}

#[test]
fn down_sampling_also_keeps_its_duration() {
    // 48 kHz down to 44.1 kHz: the direction that discards samples in a cheap converter.
    let input = tone(1_000.0, MODEL_SAMPLE_RATE, 1.0);
    let out = convert(&input, MODEL_SAMPLE_RATE, 44_100).expect("converts");
    assert_eq!(out.len(), 44_100, "one second stays one second downward too");
    let c = crossings(&out);
    assert!((998..=1002).contains(&c), "and the tone is still 1 kHz; got {c}");
}

#[test]
fn the_models_rate_is_the_identity() {
    let input = tone(440.0, MODEL_SAMPLE_RATE, 0.1);
    let out = to_model_rate(&input, MODEL_SAMPLE_RATE).expect("no conversion needed");
    assert_eq!(out, input, "no arithmetic when no conversion is needed");
}

#[test]
fn the_shortcut_to_the_model_rate_does_the_same_conversion() {
    let input = tone(1_000.0, 44_100, 0.25);
    let direct = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    let helper = to_model_rate(&input, 44_100).expect("converts");
    assert_eq!(direct, helper, "one path to the model's rate, not two");
}

#[test]
fn a_zero_rate_is_refused_and_an_empty_buffer_is_not_an_error() {
    assert_eq!(convert(&[0.1], 0, 48_000), Err(BufferProblem::ZeroSampleRate));
    assert_eq!(convert(&[0.1], 48_000, 0), Err(BufferProblem::ZeroSampleRate));
    assert_eq!(convert(&[], 44_100, 48_000), Ok(vec![]), "an empty file converts to nothing");
}

#[test]
fn the_conversion_is_deterministic() {
    let input = tone(1_000.0, 44_100, 0.3);
    let a = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    let b = convert(&input, 44_100, MODEL_SAMPLE_RATE).expect("converts");
    assert_eq!(a, b, "the same buffer converts to the same buffer, bit for bit");
}
