//! **The evidence harness's own contract**, tested where a test can reach it.
//!
//! `examples/render/shapes.rs` is a module of the render example, so this includes it directly — `#[path]` rather than a
//! copy, so the test and the harness cannot drift. Running the example itself would take nine minutes per test and
//! prove less: these assert the properties the harness's own doc claims.

#[allow(dead_code)]
#[path = "../examples/render/shapes.rs"]
mod shapes;

use shapes::{RATE, at_speed, timbre, wave};

/// A deterministic hash of a rendered buffer, the way the harness writes it: 16-bit little-endian at a gain.
fn bytes(samples: &[f32], gain: f32) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|s| {
            let scaled = (s * gain).clamp(-1.0, 1.0);
            ((scaled * f32::from(i16::MAX)) as i16).to_le_bytes()
        })
        .collect()
}

#[test]
fn every_cue_name_gets_its_own_timbre() {
    // The defect: every cue was `wave(440.0 * ratio, ..)`, so three cues shared one file. The table's own sound names,
    // each at the gain and length the harness gives it.
    let sounds = [
        "step_01",
        "step_02",
        "step_03",
        "stone_01",
        "burst_01",
        "click",
        "line_01",
        "engine_loop",
        "bang_01",
    ];
    let mut seen: Vec<(Vec<u8>, &str)> = Vec::new();
    for sound in sounds {
        let (base, second, third, decay) = timbre(sound, false);
        let rendered = bytes(&wave(base, second, third, 320, decay), 1.0);
        if let Some((_, other)) = seen.iter().find(|(b, _)| *b == rendered) {
            panic!("`{sound}` and `{other}` are byte-identical — every cue must be its own sound");
        }
        seen.push((rendered, sound));
    }
    assert_eq!(seen.len(), sounds.len(), "all distinct");
}

#[test]
fn a_loop_is_a_body_and_an_effect_is_a_transient() {
    // A loop decays slowly so its spectrogram is a sustained body; an effect decays fast so its is a transient. The
    // figure is the tail's peak as a fraction of the body's, read from the samples rather than from a picture.
    let tail = |sound: &str, is_loop: bool| -> f32 {
        let (base, second, third, decay) = timbre(sound, is_loop);
        let s = wave(base, second, third, if is_loop { 1500 } else { 320 }, decay);
        let start = s.len() * 9 / 10;
        s[start..].iter().fold(0.0f32, |m, x| m.max(x.abs()))
    };
    let body = tail("music_calm", true);
    let effect = tail("bang_01", false);
    assert!(body > 0.01, "a loop is still sounding at its end: {body:.4}");
    assert!(
        effect < body / 4.0,
        "an effect has decayed by its end: {effect:.4} against the loop's {body:.4}"
    );
}

#[test]
fn a_pitch_is_a_playback_speed_and_lengthens_the_file() {
    // `0 dB, speed 0.9439` in `render.txt` is a cue detuned down a semitone. At that rate a 320 ms effect becomes
    // 339 ms, and the file's length must show it — the harness discarded `speed` and wrote every cue at 1.0.
    let (base, second, third, decay) = timbre("step_01", false);
    let authored = wave(base, second, third, 320, decay);
    let played = at_speed(&authored, 0.9439);
    let expected = ((authored.len() as f32) / 0.9439).round() as usize;
    assert_eq!(played.len(), expected, "a slower speed is a longer file");
    assert!(played.len() > authored.len(), "{} vs {}", played.len(), authored.len());
}

#[test]
fn a_pitch_also_lowers_every_frequency() {
    // The half of a playback rate a length check misses: the tone itself drops. Counting rising zero crossings over a
    // known span reads the frequency back without a spectrogram.
    let (base, second, third, decay) = timbre("step_01", false);
    let authored = wave(base, second, third, 320, decay);
    let crossings = |s: &[f32]| s.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();

    let down_one_semitone = at_speed(&authored, 0.9439);
    let a = crossings(&authored) as f32 / authored.len() as f32;
    let b = crossings(&down_one_semitone) as f32 / down_one_semitone.len() as f32;
    // Per sample, a slower playback crosses zero *less* often by the speed factor.
    assert!((b / a - 0.9439).abs() < 0.02, "the tone drops by the speed factor: {b:.5} vs {a:.5}");
}

#[test]
fn speed_one_is_the_identity_and_a_bad_speed_is_not_a_panic() {
    let (base, second, third, decay) = timbre("bang_01", false);
    let authored = wave(base, second, third, 320, decay);
    assert_eq!(at_speed(&authored, 1.0), authored, "no arithmetic when no change is asked for");
    assert_eq!(at_speed(&authored, 0.0), authored, "a zero speed falls back rather than dividing by it");
    assert_eq!(at_speed(&authored, f32::NAN), authored, "and so does a NaN");
    assert_eq!(at_speed(&[], 0.5), Vec::<f32>::new(), "an empty buffer stays empty");
}

#[test]
fn the_buffer_is_the_length_the_rate_and_duration_say() {
    // 320 ms at 22.05 kHz is 7056 samples, which is what `ffmpeg` then reports as 0.32 s.
    let (base, second, third, decay) = timbre("step_01", false);
    assert_eq!(wave(base, second, third, 320, decay).len(), (RATE as usize) * 320 / 1000);
    assert_eq!(wave(base, second, third, 1500, decay).len(), (RATE as usize) * 1500 / 1000);
}

#[test]
fn a_timbre_is_stable_for_a_name_and_differs_across_names() {
    // FNV-1a over the name, so two runs produce the same cue — the crate's determinism rule applied to its evidence.
    assert_eq!(timbre("step_01", false), timbre("step_01", false));
    assert_ne!(timbre("step_01", false), timbre("step_02", false));
    // And a loop's ratios and decay differ from an effect's even for one name.
    assert_ne!(timbre("music_calm", true).3, timbre("music_calm", false).3, "a loop decays more slowly");
}
