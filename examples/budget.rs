//! **The voice budget over time**, so "does a voice free its slot" is a line and not an impression.
//!
//! ```sh
//! cargo run --release --features mixer --example budget -- <out-dir> \
//!     [--voices 40] [--cap 32] [--life 0.05] [--priority 0] [--max-instances 0]
//! ```
//!
//! `--priority 180` is `UNSTEALABLE_PRIORITY`: the case where a voice that is never retired cannot even be stolen,
//! so a full budget refuses every later play rather than cutting an audible one short.
//!
//! A headless app whose crate-owned mixer is a scripted [`AudioOutput`]: a voice starts on its bus, and its sample
//! ends `life` seconds later. The run plays one one-shot a frame against a `max_voices` cap and writes
//!
//! - `budget.csv` — per frame: the live voice count, the plays so far, and the `VoiceLimit` refusals so far;
//! - `budget.txt` — the summary a reviewer reads: how many plays were answered `Played`, how many were refused,
//!   and the highest live count;
//! - `budget.svg` — the live count and the refusals as lines, because `showwavespic` draws audio and turns a count
//!   into an unreadable blob.
//!
//! The fake output is the point: a headless machine has no device, and what this change is about is the *decision*
//! — whether an ended voice still occupies a slot — which the crate makes identically on both backends. The
//! `mixer` feature must be on, because a voice gets a `MixerVoice` only when that backend owns playback.

#![cfg(feature = "mixer")]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy_siren::output::{AudioOutput, Backend, Output};
use bevy_siren::{
    BusLevel, Cue, CueTable, Entry, Play, Refusal, SirenLog, SirenSettings, SirenSounds, SoundSlot, Voice,
};

/// The crate's mixer, as the crate can see it: start on a bus, and answer **per voice** whether that voice's sample
/// has ended. The clock is the harness's frame count, so a voice's life is exact rather than wall-clock noise.
struct Fake {
    clock: Arc<AtomicU64>,
    life_frames: u64,
    started: Arc<Mutex<Vec<u64>>>,
}

impl AudioOutput for Fake {
    fn set_bus_db(&mut self, _: &str, _: f32) {}

    fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}

    fn stop_voice(&mut self, _: &str, _: usize) {}

    fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
        let mut started = self.started.lock().unwrap();
        let index = started.len();
        started.push(self.clock.load(Ordering::SeqCst));
        Some(index)
    }

    fn finished(&self, _: &str, voice: usize) -> bool {
        let started = self.started.lock().unwrap();
        started
            .get(voice)
            .is_none_or(|at| self.clock.load(Ordering::SeqCst).saturating_sub(*at) >= self.life_frames)
    }
}

/// One frame of the run: its index, the voices alive at the end of it, plays answered so far, and `VoiceLimit`
/// refusals so far. A named type rather than a four-tuple, so the plot and the CSV cannot disagree about which
/// column they mean.
#[derive(Clone, Copy)]
struct Row {
    frame: u32,
    live: usize,
    played: usize,
    refused: usize,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| ".".to_owned()));
    let mut plays: usize = 40;
    let mut cap: u16 = 32;
    let mut life_s: f64 = 0.05;
    let mut priority: u8 = 0;
    let mut max_instances: u16 = 0;
    let mut keys = args;
    while let Some(key) = keys.next() {
        let value = keys.next().unwrap_or_default();
        match key.as_str() {
            "--voices" => plays = value.parse().unwrap_or(plays),
            "--cap" => cap = value.parse().unwrap_or(cap),
            "--life" => life_s = value.parse().unwrap_or(life_s),
            "--priority" => priority = value.parse().unwrap_or(priority),
            "--max-instances" => max_instances = value.parse().unwrap_or(max_instances),
            other => eprintln!("ignoring unknown argument `{other}`"),
        }
    }

    let frame_s = 1.0 / 60.0;
    let settings = SirenSettings {
        max_voices: cap,
        buses: vec![BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_secs_f64(
            frame_s,
        )))
        .add_plugins(bevy_siren::SirenPlugin)
        .insert_resource(settings);

    let clock = Arc::new(AtomicU64::new(0));
    let life_frames = (life_s / frame_s).round().max(1.0) as u64;
    app.insert_resource(Output(Box::new(Fake {
        clock: clock.clone(),
        life_frames,
        started: Arc::new(Mutex::new(Vec::new())),
    })));
    app.insert_resource(Backend(true));

    let mut sounds = SirenSounds::new();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    sounds.insert("hit", handle);
    app.insert_resource(sounds);
    app.insert_resource(CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("hit")],
        priority,
        max_instances,
        ..Cue::new("hit")
    }]));

    // Each play ends its own sample `life` after it started; a voice the crate never retires is the only way the
    // count can rise past the cap, and the only way a play can be refused with room on the bus.
    let mut rows: Vec<Row> = Vec::new();
    let mut refused_limit = 0usize;
    let mut played = 0usize;
    app.update();
    for frame in 0..u32::try_from(plays + 40).unwrap_or(u32::MAX) {
        clock.store(u64::from(frame), Ordering::SeqCst);
        if (frame as usize) < plays {
            app.world_mut().write_message(Play::cue("hit"));
        }
        app.update();
        for answer in app.world_mut().resource_mut::<Messages<bevy_siren::Played>>().drain() {
            let _ = answer;
            played += 1;
        }
        refused_limit += app
            .world_mut()
            .resource_mut::<Messages<bevy_siren::Refused>>()
            .drain()
            .filter(|r| matches!(r.reason, Refusal::VoiceLimit { .. }))
            .count();
        let live = app.world_mut().query::<&Voice>().iter(app.world()).count();
        rows.push(Row { frame, live, played, refused: refused_limit });
    }

    let peak = rows.iter().map(|r| r.live).max().unwrap_or(0);
    let ended_lines = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why == "ended"))
        .count();

    let _ = std::fs::create_dir_all(&out);
    let mut csv = String::from("frame,live,played,refused\n");
    for row in &rows {
        csv.push_str(&format!("{},{},{},{}\n", row.frame, row.live, row.played, row.refused));
    }
    std::fs::write(out.join("budget.csv"), csv).expect("budget.csv is writable");

    let summary = format!(
        "plays asked: {plays}\nplayed: {played}\nrefused VoiceLimit: {refused_limit}\npeak live voices: {peak}\ncap: {cap}\npriority: {priority}\nmax_instances: {max_instances}\nvoice mode: Once, sample life {life_s:.3} s\nended lines in the log: {ended_lines}\nlast frame live voices: {}\n",
        rows.last().map_or(0, |r| r.live),
    );
    std::fs::write(out.join("budget.txt"), &summary).expect("budget.txt is writable");
    print!("{summary}");

    // **Each of the three hard caps, forced, so the sentence a panel prints is the evidence.** They are three different
    // things a project tunes (Gregory §14.5.8.4 sets a group's limit and the engine's separately), and a refusal must
    // keep them apart: the bus is named only when the bus is what refused.
    let mut probe = String::new();
    for (label, settings, table, first, second) in cap_probes() {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<AudioSource>()
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(16),
            ))
            .add_plugins(bevy_siren::SirenPlugin)
            .insert_resource(settings)
            .insert_resource(table);
        let mut sounds = SirenSounds::new();
        // A **real** WAV: the crate's own mixer decodes the bytes, and four zero bytes are not a WAV, so a dummy asset
        // would be refused by `start_pending` (as `MissingSound { sound: <cue> }`) before the budget is ever consulted.
        for key in ["line", "bed"] {
            let handle = app
                .world_mut()
                .resource_mut::<Assets<AudioSource>>()
                .add(AudioSource { bytes: Arc::from(wav_bytes(440.0, 40, 0.0)) });
            sounds.insert(key, handle);
        }
        app.insert_resource(sounds);
        app.world_mut().write_message(Play::cue(first));
        app.update();
        // The first play must not be refused; only the second is the cap's answer.
        let first_said: Vec<String> = app
            .world_mut()
            .resource_mut::<Messages<bevy_siren::Refused>>()
            .drain()
            .map(|r| r.reason.to_string())
            .collect();
        let _ = &first_said;
        app.world_mut().write_message(Play::cue(second));
        app.update();
        let said = app
            .world_mut()
            .resource_mut::<Messages<bevy_siren::Refused>>()
            .drain()
            .next()
            .map(|r| format!("{:?} -> {}", r.reason, r.reason))
            .unwrap_or_else(|| "not refused".to_owned());
        probe.push_str(&format!("{label}: {said}\n"));
    }
    std::fs::write(out.join("refusals.txt"), &probe).expect("refusals.txt is writable");
    std::fs::write(out.join("missing-sound.txt"), missing_sound_probe())
        .expect("missing-sound.txt is writable");
    std::fs::write(out.join("budget.svg"), svg(&rows, cap)).expect("budget.svg is writable");
    print!("{probe}");
    println!("budget: {} files in {}", 4, out.display());
}

/// **The two causes of `MissingSound`, side by side**, so the sentence and the key can be judged together: a key with
/// no handle (answered by `play_requests`) and a key whose bytes the decoder refuses (answered by `start_pending`).
/// Both must name the **sound key**, never the cue.
fn missing_sound_probe() -> String {
    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("step_03")], ..Cue::new("footstep.grass") }]);
    let mut out = String::new();

    // (a) A key with no handle at all: `SirenSounds` is empty.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)))
        .add_plugins(bevy_siren::SirenPlugin)
        .insert_resource(settings.clone())
        .insert_resource(table.clone())
        .insert_resource(SirenSounds::new());
    app.update();
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();
    if let Some(r) = app.world_mut().resource_mut::<Messages<bevy_siren::Refused>>().drain().next() {
        out.push_str(&format!("no handle:     {:?} -> {}\n", r.reason, r.reason));
    }

    // (b) A key with a handle whose bytes the decoder refuses: four zeroes are not a WAV, and the mixer's `play`
    // returns `None` for exactly that.
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)))
        .add_plugins(bevy_siren::SirenPlugin)
        .insert_resource(settings)
        .insert_resource(table);
    let mut sounds = SirenSounds::new();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    sounds.insert("step_03", handle);
    app.insert_resource(sounds);
    app.insert_resource(bevy_siren::output::Output(Box::new(DecodeRefuser {
        refused: Arc::new(Mutex::new(Vec::new())),
    })));
    app.insert_resource(bevy_siren::output::Backend(true));
    app.update();
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();
    app.update();
    if let Some(r) = app.world_mut().resource_mut::<Messages<bevy_siren::Refused>>().drain().next() {
        out.push_str(&format!("undecodable:   {:?} -> {}\n", r.reason, r.reason));
    }
    out
}

/// An output whose `play` refuses every sample, which is what the crate's mixer does when the decoder rejects bytes.
struct DecodeRefuser {
    refused: Arc<Mutex<Vec<usize>>>,
}

impl AudioOutput for DecodeRefuser {
    fn set_bus_db(&mut self, _: &str, _: f32) {}
    fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}
    fn stop_voice(&mut self, _: &str, voice: usize) {
        self.refused.lock().unwrap().push(voice);
    }
    fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
        None
    }
    fn finished(&self, _: &str, _: usize) -> bool {
        true
    }
}

/// A real 16-bit mono WAV, so the crate's own mixer can decode it: `start_pending` refuses a sample it cannot decode
/// (`MissingSound`), and a probe with dummy bytes would measure that instead of the budget.
fn wav_bytes(hz: f32, ms: u32, decay: f32) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let count = (RATE as f32 * (ms as f32 / 1000.0)) as usize;
    let mut out = Vec::with_capacity(44 + count * 2);
    let data_len = (count * 2) as u32;
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        let envelope = (-t * decay).exp().min(1.0);
        let sample = (t * hz * std::f32::consts::TAU).sin() * envelope * 0.35;
        out.extend_from_slice(&((sample * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

/// The three caps, each forced with the smallest table that reaches it.
fn cap_probes() -> Vec<(&'static str, SirenSettings, CueTable, &'static str, &'static str)> {
    let unstealable = 200;
    vec![
        (
            "table budget",
            SirenSettings {
                max_voices: 1,
                buses: vec![BusLevel::new("Effects"), BusLevel::new("Ambience")],
                ducks: Vec::new(),
                ..SirenSettings::default()
            },
            CueTable::new(vec![
                Cue {
                    sounds: vec![SoundSlot::new("line")],
                    priority: unstealable,
                    max_instances: 0,
                    ..Cue::new("line")
                },
                Cue {
                    sounds: vec![SoundSlot::new("bed")],
                    bus: "Ambience".to_owned(),
                    priority: unstealable,
                    max_instances: 0,
                    ..Cue::new("bed")
                },
            ]),
            "line",
            "bed",
        ),
        (
            "cue polyphony",
            SirenSettings {
                max_voices: 8,
                buses: vec![BusLevel::new("Effects")],
                ducks: Vec::new(),
                ..SirenSettings::default()
            },
            CueTable::new(vec![Cue {
                sounds: vec![SoundSlot::new("line")],
                priority: unstealable,
                max_instances: 1,
                ..Cue::new("line")
            }]),
            "line",
            "line",
        ),
        (
            "bus voices",
            SirenSettings {
                max_voices: 8,
                buses: vec![BusLevel::new("Effects").max_voices(1)],
                ducks: Vec::new(),
                ..SirenSettings::default()
            },
            CueTable::new(vec![Cue {
                sounds: vec![SoundSlot::new("line")],
                priority: unstealable,
                max_instances: 0,
                ..Cue::new("line")
            }]),
            "line",
            "line",
        ),
    ]
}

/// The live count and the refusal count as two lines over the run, with a legend and axis labels, so a reader can
/// name either line without the code. Colours are the reader's own (`var(--c1)`, `var(--c4)`) so the plot fits a
/// light or a dark theme.
fn svg(rows: &[Row], cap: u16) -> String {
    let w = 900.0f64;
    let h = 320.0f64;
    let left = 64.0f64;
    let right = 32.0f64;
    let top = 88.0f64;
    let bottom = 56.0f64;
    let n = rows.len().max(2) as f64;
    // The axis always shows the cap, so a run that never reached it still reads against it; the caption reports the
    // run's own peak, which is the number a reviewer compares. A tenth of headroom keeps the cap line off the axis
    // top, where its label would collide with the axis's own.
    let run_peak = rows.iter().map(|r| r.live.max(r.refused)).max().unwrap_or(0);
    let top_value = run_peak.max(usize::from(cap)).max(1);
    let peak = (top_value as f64 * 1.12).ceil();
    let x = |i: usize| left + (i as f64 / (n - 1.0)) * (w - left - right);
    let y = |v: f64| h - bottom - (v / peak) * (h - top - bottom);
    let polyline = |pick: fn(&Row) -> usize, colour: &str| {
        let points: Vec<String> =
            rows.iter().enumerate().map(|(i, r)| format!("{:.1},{:.1}", x(i), y(pick(r) as f64))).collect();
        format!(
            "<polyline fill=\"none\" stroke=\"var({colour})\" stroke-width=\"2\" points=\"{}\"/>",
            points.join(" ")
        )
    };
    let cap_y = y(f64::from(cap));
    let axis_y = h - bottom;
    let right_x = w - right;
    let legend_y = 62.0;
    let swatch_y = legend_y - 9.0;
    // Left-anchored: a right-anchored pair put the refusals swatch outside the viewBox, so one of the two lines in
    // the picture had no entry in the legend at all.
    let live_swatch_x = left + 4.0;
    let live_label_x = left + 18.0;
    let refused_swatch_x = left + 108.0;
    let refused_label_x = left + 122.0;
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {w} {h}\" width=\"{w}\" height=\"{h}\">"
    ));
    out.push_str(&format!(
        "<text x=\"{left}\" y=\"22\" font-size=\"14\" fill=\"var(--fg)\">live voices and VoiceLimit refusals over a run of one-shots</text>"
    ));
    out.push_str(&format!(
        "<text x=\"{left}\" y=\"44\" font-size=\"11\" fill=\"var(--muted)\">cap {cap} · {n} frames · peak {run_peak} live</text>"
    ));
    out.push_str(&format!(
        "<rect x=\"{live_swatch_x}\" y=\"{swatch_y}\" width=\"10\" height=\"10\" fill=\"var(--c1)\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{live_label_x}\" y=\"{legend_y}\" font-size=\"11\" fill=\"var(--fg)\">live voices</text>"
    ));
    out.push_str(&format!(
        "<rect x=\"{refused_swatch_x}\" y=\"{swatch_y}\" width=\"10\" height=\"10\" fill=\"var(--c4)\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{refused_label_x}\" y=\"{legend_y}\" font-size=\"11\" fill=\"var(--fg)\">VoiceLimit refusals</text>"
    ));
    out.push_str(&format!(
        "<line x1=\"{left}\" y1=\"{axis_y:.1}\" x2=\"{right_x:.1}\" y2=\"{axis_y:.1}\" stroke=\"var(--border)\"/>"
    ));
    out.push_str(&format!(
        "<line x1=\"{left}\" y1=\"{top:.1}\" x2=\"{left}\" y2=\"{axis_y:.1}\" stroke=\"var(--border)\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\" text-anchor=\"end\">0</text>",
        left - 6.0,
        axis_y - 4.0
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\" text-anchor=\"end\">{peak:.0}</text>",
        left - 6.0,
        top + 4.0
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\" text-anchor=\"middle\">frame →</text>",
        (left + right_x) / 2.0,
        h - 20.0
    ));
    out.push_str(&format!(
        "<line x1=\"{left}\" y1=\"{cap_y:.1}\" x2=\"{right_x:.1}\" y2=\"{cap_y:.1}\" stroke=\"var(--border)\" stroke-dasharray=\"4 4\"/>"
    ));
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"11\" fill=\"var(--muted)\" text-anchor=\"end\">cap {cap}</text>",
        right_x - 4.0,
        cap_y - 5.0
    ));
    out.push_str(&polyline(|r| r.live, "--c1"));
    out.push_str(&polyline(|r| r.refused, "--c4"));
    out.push_str("</svg>\n");
    out
}
