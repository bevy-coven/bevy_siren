//! **The audio panel, as a program**: every bus with its level, its duck and its live voice count; the cue
//! table; the log of what played and why — driven by a scripted run so a capture shows a duck, a stolen voice
//! and a music layer entering on the bar.
//!
//! ```sh
//! cargo run --release --example mixer
//! ```
//!
//! # Capture knobs
//!
//! - `SIREN_FRAMES=<n>` saves one still at frame `n` to `SIREN_SHOT` (default `mixer.png`), prints
//!   `siren: frames <n> mean_ms <x>`, and exits ten frames later.
//! - `SIREN_SHOT_DIR=<dir>` also keeps every `SIREN_SHOT_EVERY`th frame from `SIREN_SHOT_FROM` there as
//!   `f_%04d.png`, for a GIF.
//! - `SIREN_SIZE=<w>x<h>` sets the window size (default 1280x720).
//! - `SIREN_KEYS=steal,layer,duck` stages one step every `SIREN_KEY_EVERY` frames (default 20) from frame 30, cycling:
//!   `duck` plays a line of dialogue, `steal` fills the bus capped at two, `layer` brings the combat layer in on
//!   the next bar.
//!
//! # Why the sounds are synthesised
//!
//! A capture has to run on a machine with no audio assets as well as no audio device, and the crate's point is
//! the decisions, not the samples. So every sound is a 16-bit mono WAV built at startup ([`wav_bytes`]) — a sine
//! with a decay for effects, a tone for each music layer — inserted into [`SirenSounds`] the same way a game
//! inserts its loaded files.

use bevy::audio::{AudioSource, SpatialListener};
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::text::FontSize;
use bevy::ui::widget::Text;
use bevy::window::WindowResolution;
use bevy_siren::{
    BusLevel, Cue, CueTable, Entry, Mode, MusicLayer, MusicLayers, MusicPlugin, MusicScore, Play, SirenLog,
    SirenMix, SirenPlugin, SirenSettings, SirenSounds, SoundSlot,
};

/// The capture, present only when `SIREN_FRAMES` is set.
#[derive(Resource)]
struct Capture {
    at: u32,
    frame: u32,
    shot: String,
    dir: Option<String>,
    from: u32,
    every: u32,
    kept: u32,
    keys: Vec<Step>,
    key_every: u32,
    total_ms: f64,
    counted: u32,
}

/// One staged step of a capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Duck,
    Steal,
    Layer,
}

/// The frame staged steps start on: after the layout and the first mix have settled.
const KEY_FROM: u32 = 30;

/// The panel's text nodes, rebuilt in place each frame.
#[derive(Component)]
struct PanelText(Kind);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Bus(&'static str),
    Log,
    Cues,
    Header,
    BusesHeading,
    CuesHeading,
    LogHeading,
}

/// A body line: one `Text` a panel section fills.
fn row(kind: Kind) -> impl Bundle {
    (
        Text::new(""),
        TextFont { font_size: FontSize::Px(17.0), ..default() },
        TextColor(Color::srgb(0.74, 0.79, 0.90)),
        PanelText(kind),
    )
}

/// A section title.
fn heading(label: &str, kind: Kind) -> impl Bundle {
    let size = if kind == Kind::Header { 30.0 } else { 18.0 };
    let ink = if kind == Kind::Header { Color::srgb(0.92, 0.94, 1.0) } else { Color::srgb(0.55, 0.62, 0.75) };
    (
        Text::new(label.to_owned()),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(ink),
        PanelText(kind),
    )
}

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "bevy_siren: the audio panel".into(),
            resolution: window_size(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((SirenPlugin, MusicPlugin))
    .insert_resource(settings())
    .insert_resource(cues())
    .insert_resource(score())
    .init_resource::<Clock>()
    .add_systems(Startup, setup)
    .add_systems(Update, (draw_panel, stage, capture).chain());

    if std::env::var("SIREN_FRAMES").is_ok() {
        app.insert_resource(capture_config());
    }
    app.run();
}

/// The frames this run has drawn, for the capture's own accounting.
#[derive(Resource, Default)]
struct Clock {
    frame: u32,
}

fn window_size() -> WindowResolution {
    match std::env::var("SIREN_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
    {
        Some((Ok(w), Ok(h))) => WindowResolution::new(w, h).with_scale_factor_override(1.0),
        _ => WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
    }
}

/// The buses the panel draws: two with room, one capped at two so a steal is visible, and the default dialogue
/// duck between them.
fn settings() -> SirenSettings {
    SirenSettings {
        max_voices: 12,
        buses: vec![
            BusLevel::new("Music").gain_db(-2.0),
            BusLevel::new("Effects").max_voices(2),
            BusLevel::new("Ambience").gain_db(-6.0),
            BusLevel::new("Dialogue"),
        ],
        ..SirenSettings::default()
    }
}

/// The cues the panel lists, and the keys their samples are synthesised under.
fn cues() -> CueTable {
    CueTable::new(vec![
        Cue {
            sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
            priority: 10,
            max_instances: 2,
            ..Cue::new("footstep.grass")
        },
        Cue {
            sounds: vec![SoundSlot::new("hit_01")],
            bus: "Effects".to_owned(),
            priority: 5,
            gain_db: -2.0,
            ..Cue::new("sword.hit")
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
        Cue { sounds: vec![SoundSlot::new("click")], priority: 20, gain_db: -6.0, ..Cue::new("ui.click") },
        Cue {
            sounds: vec![SoundSlot::new("line_01"), SoundSlot::new("line_02")],
            bus: "Dialogue".to_owned(),
            priority: 200,
            ..Cue::new("dialogue.greeting")
        },
        Cue {
            sounds: vec![SoundSlot::new("wind_loop")],
            bus: "Ambience".to_owned(),
            mode: Mode::Loop,
            priority: 100,
            gain_db: -10.0,
            ..Cue::new("ambience.wind")
        },
        Cue {
            sounds: vec![SoundSlot::new("music_calm")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("track.calm")
        },
        Cue {
            sounds: vec![SoundSlot::new("music_combat")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("track.combat")
        },
        Cue {
            sounds: vec![SoundSlot::new("stinger")],
            bus: "Music".to_owned(),
            priority: 210,
            ..Cue::new("music.stinger")
        },
    ])
}

fn score() -> MusicScore {
    MusicScore {
        layers: vec![
            MusicLayer::new("calm", "track.calm").at_db(-4.0),
            MusicLayer::new("combat", "track.combat"),
        ],
        tempo_bpm: 160.0,
        beats_per_bar: 4.0,
    }
}

fn setup(mut commands: Commands, mut sounds: ResMut<SirenSounds>, mut assets: ResMut<Assets<AudioSource>>) {
    commands.spawn((Camera2d, SpatialListener::new(1.0)));

    // Every key the table names, as a synthesised sample: the panel is about decisions, not recordings.
    let keys: [(&str, f32, u32, f32); 11] = [
        ("step_01", 220.0, 120, 30.0),
        ("step_02", 260.0, 120, 30.0),
        ("hit_01", 90.0, 260, 18.0),
        ("engine_loop", 55.0, 1_500, 0.0),
        ("click", 1_100.0, 60, 60.0),
        ("line_01", 330.0, 320, 6.0),
        ("line_02", 392.0, 320, 6.0),
        ("wind_loop", 70.0, 1_200, 1.0),
        ("music_calm", 196.0, 2_000, 1.0),
        ("music_combat", 233.0, 1_000, 1.0),
        ("stinger", 523.0, 300, 14.0),
    ];
    for (key, hz, ms, decay) in keys {
        let handle = assets.add(AudioSource { bytes: wav_bytes(hz, ms, decay).into() });
        sounds.insert(key, handle);
    }

    // The panel: a title, a row per bus, the cue list, and the log.
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::all(Val::Px(24.0)),
            row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.07, 0.09)),
        children![
            heading("bevy_siren: the mix", Kind::Header),
            heading("Buses", Kind::BusesHeading),
            row(Kind::Bus("Music")),
            row(Kind::Bus("Effects")),
            row(Kind::Bus("Ambience")),
            row(Kind::Bus("Dialogue")),
            heading("Cues", Kind::CuesHeading),
            row(Kind::Cues),
            heading("Log", Kind::LogHeading),
            row(Kind::Log),
        ],
    ));
}

/// A 16-bit mono 22.05 kHz WAV: a sine at `hz` for `ms` milliseconds, decaying by `decay` per second.
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
        // `decay` 0 is a flat tone: the envelope is 1 and the loop does not fade.
        let envelope = (-t * decay).exp().min(1.0);
        let sample = (t * hz * std::f32::consts::TAU).sin() * envelope * 0.35;
        out.extend_from_slice(&((sample * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

/// The panel's numbers, from [`SirenMix`] and [`SirenLog`] — the same resources a game's meter reads.
///
/// A heading is static text and is never rewritten; a bus row owns one bus; the cue list and the log own their
/// blocks. Nothing is duplicated: the screenshot showed the whole cue list three times when three nodes shared one
/// `Kind::Cues`, which is exactly the kind of defect a capture exists to catch.
fn draw_panel(
    mix: Res<SirenMix>,
    table: Res<CueTable>,
    settings: Res<SirenSettings>,
    log: Res<SirenLog>,
    mut texts: Query<(&PanelText, &mut Text)>,
) {
    for (panel, mut text) in &mut texts {
        let next = match panel.0 {
            Kind::Header => format!(
                "bevy_siren: {} cues, {} buses, {} voices",
                table.cues.len(),
                settings.buses.len(),
                mix.voices.values().sum::<usize>()
            ),
            Kind::BusesHeading | Kind::CuesHeading | Kind::LogHeading => continue,
            Kind::Bus(bus) => {
                let db = mix.bus_db(bus);
                let duck = mix.ducked.get(bus).copied().unwrap_or(0.0);
                let duck = if duck < -0.05 { format!("   duck {duck:+.1} dB") } else { String::new() };
                let cap = settings.bus_voice_limit(bus);
                format!("{bus:<10} {db:+7.1} dB{duck}   {} / {cap} voices", mix.voices_on(bus))
            }
            Kind::Cues => table
                .cues
                .iter()
                .map(|cue| {
                    let mode = if cue.mode.is_loop() { "loop" } else { "once" };
                    format!(
                        "{:<20} {:<10} {mode:<4} {:+5.1} dB  priority {}",
                        cue.name,
                        cue.bus_name(),
                        cue.gain_db,
                        cue.priority
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
            Kind::Log => log.recent(10).map(Entry::line).collect::<Vec<_>>().join("\n"),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
}

/// `SIREN_KEYS`: one staged step every `SIREN_KEY_EVERY` frames, from `KEY_FROM`.
fn stage(
    capture: Option<Res<Capture>>,
    mut plays: MessageWriter<Play>,
    mut layers: MessageWriter<MusicLayers>,
) {
    let Some(c) = capture else {
        return;
    };
    let Some(since) = c.frame.checked_sub(KEY_FROM) else {
        return;
    };
    let every = c.key_every.max(2);
    if since % every != 0 {
        return;
    }
    // The steps cycle, so a long capture keeps showing the three things the panel is for: the last one before the
    // still is the duck, which is still holding when the screenshot is taken.
    let Some(step) = c.keys.get(((since / every) as usize) % c.keys.len()) else {
        return;
    };
    match step {
        Step::Duck => {
            plays.write(Play::cue("dialogue.greeting"));
        }
        Step::Steal => {
            // The `Effects` bus is capped at two: two idle loops fill it, and the hit takes the oldest back, so the
            // bus stays at two and the log keeps `stolen by sword.hit` as the evidence.
            plays.write(Play::cue("engine.idle"));
            plays.write(Play::cue("engine.idle"));
            plays.write(Play::cue("sword.hit"));
        }
        Step::Layer => {
            layers.write(
                MusicLayers::new(vec![
                    ("combat".to_owned(), -3.0),
                    ("calm".to_owned(), bevy_siren::model::SILENT_DB),
                ])
                .next_bar(),
            );
        }
    }
}

fn capture_config() -> Capture {
    let at = std::env::var("SIREN_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(120);
    // **A capture with no `SIREN_KEYS` uses the documented default script.** An empty list used to reach
    // `% c.keys.len()` in `stage_keys` and panic with *"attempt to calculate the remainder with a divisor of zero"*
    // on the **default** run — the very command the module doc tells a person to type. The default is the three
    // things the panel exists to show, in the order the doc names them.
    let keys: Vec<Step> = std::env::var("SIREN_KEYS")
        .unwrap_or_else(|_| "steal,layer,duck".to_owned())
        .split(',')
        .filter_map(|token| match token.trim() {
            "duck" => Some(Step::Duck),
            "steal" => Some(Step::Steal),
            "layer" => Some(Step::Layer),
            _ => None,
        })
        .collect();
    let keys = if keys.is_empty() {
        // **Louder than a silent fallback:** a token nobody recognises is a typo, and a capture that quietly staged
        // something else would put the wrong picture in the evidence folder.
        if std::env::var("SIREN_KEYS").is_ok_and(|v| !v.trim().is_empty()) {
            eprintln!(
                "siren: SIREN_KEYS has no step this example knows; staging the default steal,layer,duck"
            );
        }
        vec![Step::Steal, Step::Layer, Step::Duck]
    } else {
        keys
    };
    Capture {
        at,
        frame: 0,
        shot: std::env::var("SIREN_SHOT").unwrap_or_else(|_| "mixer.png".into()),
        dir: std::env::var("SIREN_SHOT_DIR").ok(),
        from: std::env::var("SIREN_SHOT_FROM").ok().and_then(|n| n.parse().ok()).unwrap_or(30),
        every: std::env::var("SIREN_SHOT_EVERY").ok().and_then(|n| n.parse().ok()).unwrap_or(2),
        kept: 0,
        keys,
        key_every: std::env::var("SIREN_KEY_EVERY").ok().and_then(|n| n.parse().ok()).unwrap_or(20),
        total_ms: 0.0,
        counted: 0,
    }
}

fn capture(
    capture: Option<ResMut<Capture>>,
    mut clock: ResMut<Clock>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
    mut commands: Commands,
) {
    clock.frame += 1;
    let Some(mut c) = capture else {
        return;
    };
    c.frame = clock.frame;
    if c.frame > 20 && c.frame <= c.at {
        c.total_ms += time.delta_secs_f64() * 1000.0;
        c.counted += 1;
    }
    if c.frame == c.at {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(c.shot.clone()));
        let mean = if c.counted > 0 { c.total_ms / f64::from(c.counted) } else { 0.0 };
        println!("siren: frames {} mean_ms {mean:.3}", c.at);
    }
    if let Some(dir) = c.dir.clone() {
        let every = c.every.max(1);
        if c.frame >= c.from && (c.frame - c.from) % every == 0 {
            c.kept += 1;
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(format!("{dir}/f_{:04}.png", c.kept)));
        }
        if c.frame == c.at {
            println!("siren: kept {} frames in {dir}", c.kept);
        }
    }
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}
