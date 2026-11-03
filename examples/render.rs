//! **Every cue and bus rendered to WAV, so a change can be judged without an ear.**
//!
//! ```sh
//! cargo run --release --example render -- <out-dir> [--voices 8|64|256]
//! ```
//!
//! A headless run with no `AudioPlugin` and no `SirenPlugin`: the harness does not need the ECS path a game uses, it
//! needs the *numbers* the crate decides with, and those are a pure function of the cue table and the mix table
//! (`bevy_siren::model`). So it drives the same functions the crate's systems call and writes what a player would
//! hear:
//!
//! - `cue-<name>-<variant>.wav` — the authored sample, synthesised to stand in for a recording, at the gain and speed
//!   the play would use. One per case in `docs/loop/cues.yaml`; a refused case writes `refusal-<case>.txt` instead,
//!   because a refusal is a decision and not a sound.
//! - `bus-<name>.wav` — the bus's gain over time for the iteration's script: a duck arriving, holding and recovering,
//!   at the curve `model::active_duck_db` and `model::approach` actually produce.
//! - `mix.wav` — the sum of the cues at the gains the mix would give them, for a loudness check of the whole.
//! - `voices-<n>.wav` — `n` simultaneous voices, for the frame-cost and clipping checks in `perf.md`.
//!
//! Everything is 16-bit mono 22.05 kHz so `ffmpeg`'s `ebur128`, `showspectrumpic` and `showwavespic` read it without a
//! conversion, and so a WAV's bytes are small enough to keep in `evidence/`.
//!
//! The harness prints one line per file with the numbers a reviewer compares: integrated LUFS is `ffmpeg`'s job and
//! the command in `docs/loop/runs/README.md` prints it; the harness prints what *it* knows — peak, RMS, the gain in dB
//! at each scripted moment, and whether a refusal happened.

use std::path::{Path, PathBuf};

use bevy_siren::model::{
    active_duck_db, approach, db_to_gain, distance_gain, next_boundary, semitones_to_ratio, sum_db,
};
use bevy_siren::{Attenuation, Cue, CueTable, Duck, Mode, PitchRange, Repeat, SirenSettings, SoundSlot};

const RATE: u32 = 22_050;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(out) = args.get(1).map(PathBuf::from) else {
        eprintln!("usage: cargo run --release --example render -- <out-dir> [--voices 8|64|256]");
        eprintln!();
        eprintln!(
            "  <out-dir>  where the WAVs and the numbers go; `docs/loop/runs/<iteration>/evidence/after` in a run"
        );
        std::process::exit(2);
    };
    let voices: usize = args
        .iter()
        .position(|a| a == "--voices")
        .and_then(|i| args.get(i + 1))
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("cannot make {}: {e}", out.display());
        std::process::exit(1);
    }

    let settings = SirenSettings::default();
    let table = table();
    let mut report = Vec::new();

    // 1. One file per cue and variant, at the gain and speed a play gives it.
    for cue in &table.cues {
        for (i, slot) in cue.sounds.iter().enumerate() {
            let semitones = cue.pitch.at(0.0);
            let gain_db = gain_of(cue, &slot.sound, 0.0);
            let ms = if cue.mode.is_loop() { 1_500 } else { 320 };
            let samples = wave(440.0 * semitones_to_ratio(semitones), ms, 12.0);
            write_gain(
                &out,
                &format!("cue-{}-{i}.wav", key(&cue.name)),
                &samples,
                semitones_to_ratio(semitones),
                gain_db,
            );
            let at = stats(&samples);
            report.push(format!(
                "cue-{}-{i}: {gain_db:.2} dB, speed {:.4}, peak {:.3}, rms {:.3}",
                key(&cue.name),
                semitones_to_ratio(semitones),
                at.0,
                at.1
            ));
        }
    }

    // 2. The refusal cases: a decision, written down rather than sounded.
    for (name, refusal) in refusals(&table, &settings) {
        let path = out.join(format!("refusal-{}.txt", key(&name)));
        let _ = std::fs::write(&path, format!("{refusal}\n"));
        report.push(format!("refusal-{}: {refusal}", key(&name)));
    }

    // 3. A duck's envelope on the music bus, at 30 fps, from the default rules.
    //
    // **The envelope is a function of time since the duck started, not of the score's clock.** A dialogue line starting
    // at t = 1.0 s means the arguments to `active_duck_db` run from 0 from that frame on — which is what the crate's
    // `DuckClock` stores, a start instant it subtracts. The first version of this harness passed the wall clock and the
    // duck never recovered; the curve was right and its input was not.
    let starts_at = 1.0f64;
    let rule = settings.ducks.first().cloned().unwrap_or_else(|| Duck::new("Dialogue", "Music"));
    let mut gain = 0.0f32;
    let mut env = Vec::new();
    for frame in 0..120 {
        let t = f64::from(frame) / 30.0;
        let since = t - starts_at;
        let target =
            if since < 0.0 { 0.0 } else { active_duck_db(rule.by_db, since, rule.hold_s, rule.fade_s) };
        gain = approach(gain, target, 0.05, 1.0 / 30.0);
        env.push(gain);
    }
    let music = settings.bus("Music").map_or(0.0, |b| b.gain_db);
    // **One sample per frame is a 4 ms file.** A bus's gain is a step function between frames, so each frame's gain
    // is repeated for the samples that frame lasts: 30 fps at 22.05 kHz is 735 samples.
    let per_frame = (RATE / 30) as usize;
    let bus: Vec<f32> =
        env.iter().flat_map(|d| std::iter::repeat_n(db_to_gain(music + d), per_frame)).collect();
    write_samples(&out, "bus-Music.wav", &bus, 1.0);
    let deepest = env.iter().copied().fold(0.0f32, f32::min);
    // Search only after the duck has fired: before it the music is at 0 dB, which is not a recovery.
    let fired_at = (starts_at * 30.0) as usize;
    let recovered = env
        .iter()
        .enumerate()
        .skip(fired_at)
        .find(|(_, d)| **d > -0.05)
        .map_or(f64::NAN, |(frame, _)| frame as f64 / 30.0);
    report.push(format!(
        "bus-Music: duck {:.1} dB, deepest {deepest:.2} dB, recovered by {recovered:.2} s (fired at {starts_at:.2} s, hold {:.2} + fade {:.2})",
        rule.by_db, rule.hold_s, rule.fade_s
    ));
    for duck in settings.ducks.iter().filter(|d| d.target != "Music") {
        report.push(format!(
            "bus-{}: ducked {:.1} dB, hold {:.2} s, fade {:.2} s (from {})",
            duck.target, duck.by_db, duck.hold_s, duck.fade_s, duck.source
        ));
    }

    // 3b. **A stinger's duck**, in both readings of `duck_s`. `DuckClock::manual` takes `(…, hold_s, fade_s)`, and
    // iteration 3 found the call passing `duck_s` as the *fade* with a hold of zero, so the music began climbing on the
    // frame the stinger fired. Both envelopes are written and both numbers printed: the fix is judged by the pair, and
    // a later change that reintroduces the old one is visible as the two columns converging.
    let stinger_db = -9.0f32;
    let stinger_hold = 0.6f32;
    let stinger_release = bevy_siren::model::DEFAULT_FADE_S;
    let envelope = |hold_s: f32, fade_s: f32| {
        let mut gain = 0.0f32;
        let mut env = Vec::new();
        for frame in 0..(2.0 * 30.0) as usize {
            let t = f64::from(frame as u32) / 30.0;
            let target =
                if t < starts_at { 0.0 } else { active_duck_db(stinger_db, t - starts_at, hold_s, fade_s) };
            gain = approach(gain, target, 0.05, 1.0 / 30.0);
            env.push(gain);
        }
        env
    };
    let held = envelope(stinger_hold, stinger_release);
    let faded = envelope(0.0, stinger_hold);
    let bus_from = |env: &[f32]| -> Vec<f32> {
        env.iter().flat_map(|d| std::iter::repeat_n(db_to_gain(music + d), per_frame)).collect()
    };
    write_samples(&out, "bus-Music-stinger.wav", &bus_from(&held), 1.0);
    write_gain_plot(
        &out.join("bus-Music-stinger.svg"),
        "Music under a stinger: duck_s as the hold (the fix) and as the fade (before)",
        &[("hold then release", &held, "--c1"), ("fade from the first frame", &faded, "--c4")],
        starts_at,
    );
    write_samples(&out, "bus-Music-stinger-before.wav", &bus_from(&faded), 1.0);
    let at = |env: &[f32]| {
        env.get(((starts_at + f64::from(stinger_hold) / 2.0) * 30.0) as usize).copied().unwrap_or(0.0)
    };
    report.push(format!(
        "bus-Music-stinger: duck {stinger_db:.1} dB; duck_s as the hold ({stinger_hold:.2} s) + release {stinger_release:.2} s reads {:.2} dB halfway, duck_s as the fade reads {:.2} dB halfway",
        at(&held),
        at(&faded)
    ));

    // 4. A spatial cue's gain at the distances a player notices.
    let inverse = Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 };
    report.push(format!(
        "spatial: {:.3} at 4 m, {:.3} at 8 m, {:.3} at 16 m",
        distance_gain(inverse, 4.0),
        distance_gain(inverse, 8.0),
        distance_gain(inverse, 16.0)
    ));

    // 5. A bar, so a music transition can be checked against its tempo.
    let bar = next_boundary(0.3, 160.0, 4.0);
    report.push(format!("music: a bar at 160 bpm from 0.3 s is {bar:.2} s"));

    // 6. The mix: every cue's gain summed as the bus sum would sum it.
    let gains: Vec<f32> = table.cues.iter().map(|c| gain_of(c, &c.sounds[0].sound, 0.0) + music).collect();
    report.push(format!("mix: {} cues, {} dB summed", gains.len(), sum_db(&gains)));
    let mixed = wave(220.0, 500, 6.0);
    write_gain(&out, "mix.wav", &mixed, 1.0, sum_db(&gains));

    // 7. Many voices at once, for the clipping and frame-cost checks.
    let mut many = Vec::new();
    for i in 0..voices {
        let hz = 110.0 + 40.0 * i as f32;
        many.push(wave(hz, 400, 8.0));
    }
    let summed = sum_voices(&many);
    write_samples(&out, &format!("voices-{voices}.wav"), &summed, 1.0);
    let at = stats(&summed);
    let clipped = summed.iter().filter(|s| s.abs() > 1.0).count();
    report.push(format!(
        "voices-{voices}: peak {:.3}, rms {:.3}, {clipped} samples would clip (the budget's job, not the gain's)",
        at.0, at.1
    ));

    let lines = report.join("\n");
    let _ = std::fs::write(out.join("render.txt"), format!("{lines}\n"));
    println!("{lines}");
    println!();
    println!(
        "render: {} files in {}",
        std::fs::read_dir(&out).map(|d| d.count()).unwrap_or(0),
        out.display()
    );
}

/// A name that is a filename.
fn key(name: &str) -> String {
    name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect()
}

/// The gain a cue's variant plays at, through the same functions the play system uses.
fn gain_of(cue: &Cue, _sound: &str, request_db: f32) -> f32 {
    bevy_siren::model::chain_db(&[cue.gain_db, request_db])
}

/// The table `docs/loop/cues.yaml` describes: every case that can be a cue, authored as the case says.
fn table() -> CueTable {
    CueTable::new(vec![
        Cue {
            sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02"), SoundSlot::new("step_03")],
            repeat: Repeat::AvoidRepeat { avoid: 1 },
            pitch: PitchRange { low: -1.0, high: 1.0 },
            ..Cue::new("footstep.grass")
        },
        Cue {
            sounds: vec![SoundSlot::new("stone_01"), SoundSlot::new("stone_02")],
            repeat: Repeat::AvoidRepeat { avoid: 4 },
            ..Cue::new("footstep.stone")
        },
        Cue {
            sounds: vec![SoundSlot::new("burst_01")],
            repeat: Repeat::Shuffle,
            gain_db: -2.0,
            ..Cue::new("gun.burst")
        },
        Cue { sounds: vec![SoundSlot::new("click")], gain_db: -6.0, ..Cue::new("ui.cancel") },
        Cue {
            sounds: vec![SoundSlot::new("line_01"), SoundSlot::new("line_02")],
            bus: "Dialogue".to_owned(),
            priority: 180,
            ..Cue::new("dialogue.greeting")
        },
        Cue {
            sounds: vec![SoundSlot::new("engine_loop")],
            bus: "Effects".to_owned(),
            mode: Mode::Loop,
            priority: 5,
            max_instances: 2,
            gain_db: -14.0,
            ..Cue::new("engine.idle")
        },
        Cue {
            sounds: vec![SoundSlot::new("bang_01")],
            spatial: true,
            attenuation: Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
            ..Cue::new("bang")
        },
        Cue {
            sounds: vec![SoundSlot::new("music_calm")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("music.calm")
        },
        Cue {
            sounds: vec![SoundSlot::new("music_combat")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("music.combat")
        },
        Cue {
            sounds: vec![SoundSlot::new("stinger")],
            bus: "Music".to_owned(),
            priority: 210,
            ..Cue::new("music.stinger")
        },
    ])
}

/// The refusals `cues.yaml` asks for, written as the sentence the crate's own `Refusal` prints.
fn refusals(_table: &CueTable, settings: &SirenSettings) -> Vec<(String, String)> {
    use bevy_siren::Refusal;

    let cases = [
        ("footstep.snow", Refusal::UnknownCue),
        ("footstep.grass", Refusal::MissingSound { sound: "step_09".to_owned() }),
        ("bang", Refusal::UnknownBus { bus: "Reverb".to_owned() }),
        ("bang", Refusal::SpatiallyMute(bevy::math::Vec3::new(3.0, 0.0, 4.0))),
        ("dialogue.greeting", Refusal::VoiceLimit { bus: Some("Dialogue".to_owned()) }),
    ];
    let _ = settings;
    cases
        .into_iter()
        .map(|(cue, reason)| (format!("{cue}-{}", key(&reason.to_string())), reason.to_string()))
        .collect()
}

/// A synthesised stand-in for a recording: a sine with a decay, so a spectrogram has a readable attack and tail.
fn wave(hz: f32, ms: u32, decay: f32) -> Vec<f32> {
    let count = (RATE as f32 * (ms as f32 / 1000.0)) as usize;
    (0..count)
        .map(|i| {
            let t = i as f32 / RATE as f32;
            let envelope = (-t * decay).exp().min(1.0);
            (t * hz * std::f32::consts::TAU).sin() * envelope * 0.5
        })
        .collect()
}

/// Several voices summed the way a mixer sums them: linear, then clipped only by what a listener's device does.
fn sum_voices(voices: &[Vec<f32>]) -> Vec<f32> {
    let len = voices.iter().map(Vec::len).max().unwrap_or(0);
    (0..len).map(|i| voices.iter().filter_map(|v| v.get(i)).sum::<f32>()).collect()
}

fn write_gain(out: &Path, name: &str, samples: &[f32], speed: f32, gain_db: f32) {
    let _ = speed;
    write_samples(out, name, samples, db_to_gain(gain_db));
}

/// **A bus's gain over time as a line**, the form a gain curve can be read in. `showwavespic` draws audio and turns
/// an envelope into a blob, and a CSV of 60 rows is not a picture; this is the plot the review pass looks at.
///
/// Colours are the reader's own (`var(--c1)`, `var(--c4)`), so the same file reads on a light or a dark theme. The
/// dashed line marks the frame the duck fired.
fn write_gain_plot(path: &Path, name: &str, envelopes: &[(&str, &[f32], &str)], mark_s: f64) {
    const W: f64 = 900.0;
    const H: f64 = 320.0;
    const LEFT: f64 = 64.0;
    const RIGHT: f64 = 24.0;
    const TOP: f64 = 64.0;
    const BOTTOM: f64 = 52.0;
    let n = envelopes.iter().map(|(_, e, _)| e.len()).max().unwrap_or(2).max(2);
    let lo = envelopes.iter().flat_map(|(_, e, _)| e.iter().copied()).fold(0.0f32, f32::min).min(-1.0);
    let x = |i: usize| LEFT + (i as f64 / (n - 1) as f64) * (W - LEFT - RIGHT);
    let y =
        |v: f32| H - BOTTOM - ((f64::from(v) - f64::from(lo)) / (0.0 - f64::from(lo))) * (H - TOP - BOTTOM);
    let axis_y = H - BOTTOM;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {W} {H}\" width=\"{W}\" height=\"{H}\">"
    ));
    out.push_str(&format!("<text x=\"{LEFT}\" y=\"22\" font-size=\"14\" fill=\"var(--fg)\">{name}</text>"));
    out.push_str(&format!(
        "<text x=\"{LEFT}\" y=\"42\" font-size=\"11\" fill=\"var(--muted)\">{lo:.1} dB to 0 dB, 30 fps</text>"
    ));

    // Legend: a swatch then its label, laid out left to right so nothing can fall outside the viewBox.
    let mut legend_x = LEFT;
    for (label, _, colour) in envelopes {
        out.push_str(&format!(
            "<rect x=\"{legend_x:.1}\" y=\"{:.1}\" width=\"10\" height=\"10\" fill=\"var({colour})\"/>",
            TOP - 26.0
        ));
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--fg)\">{label}</text>",
            legend_x + 14.0,
            TOP - 17.0
        ));
        legend_x += 24.0 + label.len() as f64 * 6.2;
    }

    for (_, env, colour) in envelopes {
        let points: Vec<String> =
            env.iter().enumerate().map(|(i, v)| format!("{:.1},{:.1}", x(i), y(*v))).collect();
        out.push_str(&format!(
            "<polyline fill=\"none\" stroke=\"var({colour})\" stroke-width=\"2\" points=\"{}\"/>",
            points.join(" ")
        ));
    }

    out.push_str(&format!(
        "<line x1=\"{LEFT}\" y1=\"{TOP}\" x2=\"{LEFT}\" y2=\"{axis_y:.1}\" stroke=\"var(--border)\"/>"
    ));
    out.push_str(&format!(
        "<line x1=\"{LEFT}\" y1=\"{axis_y:.1}\" x2=\"{:.1}\" y2=\"{axis_y:.1}\" stroke=\"var(--border)\"/>",
        W - RIGHT
    ));
    let mark_x = x(((mark_s * 30.0) as usize).min(n - 1));
    out.push_str(&format!(
        "<line x1=\"{mark_x:.1}\" y1=\"{TOP}\" x2=\"{mark_x:.1}\" y2=\"{axis_y:.1}\" stroke=\"var(--border)\" stroke-dasharray=\"4 4\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{axis_y:.1}\" font-size=\"11\" fill=\"var(--muted)\">0 dB</text>",
        LEFT - 6.0
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\">{lo:.0} dB</text>",
        LEFT - 6.0,
        TOP + 4.0
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\" text-anchor=\"middle\">time (s)</text>",
        (LEFT + W - RIGHT) / 2.0,
        H - 16.0
    ));
    // Above the plot's top edge, where no curve can be: a label on the axis or in the well sits on top of the line it
    // is describing.
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\">duck fires</text>",
        mark_x + 5.0,
        TOP - 5.0
    ));
    out.push_str("</svg>\n");
    if let Err(e) = std::fs::write(path, out) {
        eprintln!("cannot write {}: {e}", path.display());
    }
}

fn write_samples(out: &Path, name: &str, samples: &[f32], gain: f32) {
    let mut bytes = Vec::with_capacity(44 + samples.len() * 2);
    let data_len = (samples.len() * 2) as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&RATE.to_le_bytes());
    bytes.extend_from_slice(&(RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        let scaled = (sample * gain).clamp(-1.0, 1.0);
        bytes.extend_from_slice(&((scaled * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    let _ = std::fs::write(out.join(name), bytes);
}

/// Peak and RMS of a buffer, so a clipping or denormal tail is a number rather than an impression.
fn stats(samples: &[f32]) -> (f32, f32) {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (samples.iter().map(|s| s * s).sum::<f32>() / samples.len().max(1) as f32).sqrt();
    (peak, rms)
}
