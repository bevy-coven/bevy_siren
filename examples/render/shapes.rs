//! **The shapes `examples/render` writes**, and the properties `tests/render_shapes.rs` pins.
//!
//! Kept beside the harness rather than in `src/`, because a synthesised stand-in for a recording is evidence tooling and
//! not something a game should be able to call. Kept in a *file* rather than inline, because the harness's own doc
//! makes two claims a test can check, and a test cannot reach a `fn` inside a binary's `main`.
//!
//! Both claims were false, and both were found by looking at the evidence rather than at the code:
//!
//! 1. **Every cue synthesised the same 440 Hz sine.** Three different cues produced one byte-identical file
//!    (`md5 eefe987d5aef348c1d6f4abe482db585`), so every spectrogram this project judged was one waveform.
//! 2. **`write_gain` discarded `speed`** (`let _ = speed;`) while the module's doc claimed it applied "the gain and the
//!    speed a play gives it".
//!
//! Nothing here touches the filesystem: these are the arithmetic, so a test can run them in microseconds instead of
//! building a release binary.

/// The rate every rendered file is written at: 22.05 kHz mono 16-bit, so `ffmpeg` reads it without a conversion.
pub const RATE: u32 = 22_050;

/// **A synthesised stand-in for a recording**, with a timbre derived from its name.
///
/// Four numbers: a fundamental, two partial ratios, and a decay. Deriving them from the sound's own name is what stops
/// a footstep being a gun burst on a spectrogram. The version before this one passed `440.0` for every cue, so every
/// cue at the same gain produced a byte-identical file.
///
/// The frequency is not the point; the *differences* are. A kick sits low and short, a footstep mid and crackly, a
/// music loop low and long — enough for a spectrogram to show the envelope each case claims, which is what the visual
/// gate is for.
pub fn timbre(sound: &str, is_loop: bool) -> (f32, f32, f32, f32) {
    // FNV-1a over the name, the same hash the library scan uses, so a name's timbre is stable across runs.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in sound.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let unit = |shift: u32| -> f32 { ((h >> shift) & 0xff) as f32 / 255.0 };
    // 90 Hz to 620 Hz: a kick sits near the bottom, a click near the top, and nothing aliases at 22.05 kHz.
    let base = 90.0 + unit(0) * 530.0;
    let second = if is_loop { 1.5 } else { 2.0 + unit(16) * 2.0 };
    let third = if is_loop { 2.0 } else { 3.0 + unit(32) * 4.0 };
    // A loop decays slowly so its spectrogram is a body rather than a transient; an effect decays fast.
    let decay = if is_loop { 0.6 + unit(48) } else { 6.0 + unit(48) * 18.0 };
    (base, second, third, decay)
}

/// A synthesised stand-in for a recording: three partials with a decay, so a spectrogram has a readable attack and a
/// tail that belongs to the sound rather than to every sound at once.
pub fn wave(base: f32, second: f32, third: f32, ms: u32, decay: f32) -> Vec<f32> {
    let count = (RATE as f32 * (ms as f32 / 1000.0)) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let envelope = (-t * decay).exp().min(1.0);
            let tau = std::f32::consts::TAU * t;
            // The second and third partials are quieter, so the fundamental is the brightest band on a spectrogram and
            // the others are texture.
            let s = tau.mul_add(base, 0.0).sin()
                + 0.45 * (tau * base * second).sin()
                + 0.22 * (tau * base * third).sin();
            s * envelope * 0.32
        })
        .collect()
}

/// **Resample for playback speed**: linear interpolation over the source at `speed` steps per output sample.
///
/// At `speed` < 1 the buffer is stretched and every frequency drops by the same factor; at `speed` > 1 it is shortened
/// and rises. That is what a rodio `speed()` does to a source, and it is why a pitch range is audible as a change of
/// length too.
///
/// A non-finite or non-positive `speed` returns the input unchanged rather than dividing by it: this runs inside the
/// evidence harness, and a NaN there would produce a file whose every sample is a zero.
pub fn at_speed(samples: &[f32], speed: f32) -> Vec<f32> {
    if !speed.is_finite() || speed <= 0.0 || samples.is_empty() {
        return samples.to_vec();
    }
    if (speed - 1.0).abs() < 1e-6 {
        return samples.to_vec();
    }
    let out_len = ((samples.len() as f32) / speed).round().max(1.0) as usize;
    let last = samples.len() - 1;
    (0..out_len)
        .map(|i| {
            let src = i as f32 * speed;
            let i0 = src.floor() as usize;
            let frac = src - i0 as f32;
            let a = samples[i0.min(last)];
            let b = samples[(i0 + 1).min(last)];
            a + (b - a) * frac
        })
        .collect()
}

/// Several voices summed the way a mixer sums them: linear, then clipped only by what a listener's device does.
pub fn sum_voices(voices: &[Vec<f32>]) -> Vec<f32> {
    let len = voices.iter().map(Vec::len).max().unwrap_or(0);
    (0..len).map(|i| voices.iter().filter_map(|v| v.get(i)).sum::<f32>()).collect()
}
