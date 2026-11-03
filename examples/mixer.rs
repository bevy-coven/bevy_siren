//! **The audio panel, as a program**: every bus with its level, its duck and its live voice count; the cue
//! table **with the crate's own verdict on it**; the log of what played and why — driven by a scripted run so a
//! capture shows a duck, a stolen voice and a music layer entering on the bar.
//!
//! ```sh
//! cargo run --release --example mixer
//! ```
//!
//! # Capture knobs
//!
//! - `SIREN_FRAMES=<n>` saves one still at frame `n` to `SIREN_SHOT` (default `mixer.png`), prints
//!   `siren: frames <n> mean_ms <x>`, and exits ten frames later.
//! - `SIREN_SHOT_DIR=<dir>` also keeps a frame every `SIREN_SHOT_EVERY` **hundredths of a second** (12 = 120 ms) from
//!   `SIREN_SHOT_FROM` there as `f_%04d.png`, for a GIF. It is a duration because what the GIF has to catch is a
//!   **fade** — `MIX_TAU_S` is 50 ms — and a frame count is not a time: sampling every twelve *updates* gave half a
//!   sample per fade.
//! - `SIREN_SIZE=<w>x<h>` sets the window size (default 1280x720).
//! - `SIREN_FAULTS=1` adds two deliberate mistakes to the cue table — a cue on a bus the mix does not have, and one
//!   whose pitch range is inverted — so a capture can show the `Problems` block and the `<- problem` markers doing
//!   something. The shipped table is clean: it is the picture everyone sees.
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
    BusLevel, Cue, CueTable, Entry, Mode, MusicLayer, MusicLayers, MusicPlugin, MusicScore, MusicState, Play,
    SirenLog, SirenMix, SirenPlugin, SirenSettings, SirenSounds, SoundSlot,
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
    /// Real time of the last frame kept for a GIF, so sampling is by **seconds**, not updates.
    kept_at_s: f64,
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
    Music,
    Problems,
    Header,
    BusesHeading,
    CuesHeading,
    MusicHeading,
    LogHeading,
    ProblemsHeading,
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
    .insert_resource(cues_with_faults())
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

/// The cues the panel lists — the shipped table, or with `SIREN_FAULTS=1`, the same table plus two deliberate mistakes,
/// so a capture can show the `Problems` block and the row markers **doing something**.
///
/// The shipped table stays clean: it is the crate's face, and a fault in the picture everyone sees would be a worse
/// picture. The fault table exists to prove the view, and is selected only by the capture knob.
fn cues_with_faults() -> CueTable {
    let mut table = cues();
    if std::env::var("SIREN_FAULTS").is_ok_and(|v| v != "0") {
        table.insert(Cue {
            sounds: vec![SoundSlot::new("hit_01")],
            bus: "Reverb".to_owned(),
            ..Cue::new("sword.hit.wrong-bus")
        });
        table.insert(Cue {
            sounds: vec![SoundSlot::new("click")],
            pitch: bevy_siren::PitchRange { low: 2.0, high: 1.0 },
            ..Cue::new("ui.click.inverted-pitch")
        });
        // `SIREN_FAULTS=many` adds four more, to see what the block does when the list is long.
        if std::env::var("SIREN_FAULTS").as_deref() == Ok("many") {
            table.insert(Cue { sounds: vec![], ..Cue::new("empty.one") });
            table.insert(Cue { sounds: vec![], ..Cue::new("empty.two") });
            table.insert(Cue {
                sounds: vec![SoundSlot::new("x")],
                bus: "Nope".to_owned(),
                ..Cue::new("bus.two")
            });
            table.insert(Cue {
                sounds: vec![SoundSlot::new("x")],
                max_instances: 0,
                ..Cue::new("unlimited")
            });
        }
    }
    table
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
            heading("Music", Kind::MusicHeading),
            row(Kind::Music),
            heading("Cues", Kind::CuesHeading),
            row(Kind::Cues),
            heading("Problems", Kind::ProblemsHeading),
            row(Kind::Problems),
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

/// Whether `problem` is about the cue called `name`.
///
/// Every `CueProblem` variant writes its sentence as ``cue `{name}` …`` — the leading token is the cue and nothing else,
/// checked against `src/cue.rs`'s `Display` — so a **prefix** match is exact and cannot be fooled by a bus name that
/// happens to look like a cue name. A match over the enum would need a new arm each time it grows; the panel is a view,
/// not a second classifier.
fn problem_names(problem: &bevy_siren::CueProblem, name: &str) -> bool {
    problem.to_string().starts_with(&format!("cue `{name}` "))
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
    music: Res<MusicState>,
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
            Kind::BusesHeading
            | Kind::CuesHeading
            | Kind::MusicHeading
            | Kind::LogHeading
            | Kind::ProblemsHeading => continue,
            Kind::Bus(bus) => {
                let db = mix.bus_db(bus);
                let duck = mix.ducked.get(bus).copied().unwrap_or(0.0);
                // **A duck says how far *and by what*.** The row used to read `duck -8.0 dB` and nothing else, so a
                // person had to find the log line that matched — while `Entry::Ducked` exists precisely to say which
                // cue lowered the bus. The newest entry naming this bus, shown while the bus is still ducked, is the
                // one a player is hearing.
                let duck = if duck < -0.05 {
                    let by = log
                        .recent(SirenLog::CAPACITY)
                        .find_map(|entry| match entry {
                            Entry::Ducked { bus: b, by, .. } if b == bus => Some(by.as_str()),
                            _ => None,
                        })
                        .unwrap_or("");
                    if by.is_empty() {
                        format!("   duck {duck:+.1} dB")
                    } else {
                        format!("   duck {duck:+.1} dB by {by}")
                    }
                } else {
                    String::new()
                };
                let cap = settings.bus_voice_limit(bus);
                format!("{bus:<10} {db:+7.1} dB{duck}   {} / {cap} voices", mix.voices_on(bus))
            }
            // **The score, as the plugin sees it.** `MusicState` exposes seven panel-facing accessors —
            // `layer_db`, `target_db`, `names`, `moving`, `live`, `next_change_s`, `voice` — and this panel drew none of
            // them, so the example shipped a two-layer score whose `Layer` step really crossfades on the bar and the
            // picture showed `track.calm` and `track.combat` as ordinary cues. One row per layer: where it is, where it
            // is going, and whether it is still moving.
            Kind::Music => {
                // **One line for the whole score.** A row per layer cost a full line more than the three cue rows it
                // displaced, and the column has no scroll — the log was clipped again the moment this block was added
                // (measured: margin 12 px -> 1 px). Two layers fit on one line in 41 of the ~114 columns available, so
                // the block costs one heading and one line and the log keeps its eight.
                let parts: Vec<String> = music
                    .names()
                    .map(|name| {
                        let db = music.layer_db(name).unwrap_or(bevy_siren::model::SILENT_DB);
                        let target = music.target_db(name).unwrap_or(db);
                        if (db - target).abs() >= 0.01 {
                            format!("{name} {db:+.1} -> {target:+.1}")
                        } else {
                            format!("{name} {db:+.1} dB")
                        }
                    })
                    .collect();
                if parts.is_empty() { "none".to_owned() } else { parts.join("   ") }
            }
            Kind::Cues => {
                // **A bad cue is marked on its own row, and the sentence lives in the `Problems` block.** The marker
                // is deliberately a short word: a full sentence here wrapped onto a second line and broke the column's
                // alignment (caught by looking at the 1280 capture), and it repeated what the block below already
                // says. One form per idea: the row says *which* cue, the block says *what*.
                let problems = table.problems_in(&settings);
                // **A cue a score owns is shown by the Music block, not here.** Three of the nine cues are music; the
                // block shows them with their live gain and target, which the static row cannot, and the column has
                // finite height — measured, three cue rows is almost exactly what a three-line block needs.
                let owned: Vec<String> = music
                    .score
                    .as_ref()
                    .map(|s| s.layers.iter().map(|l| l.cue.clone()).collect())
                    .unwrap_or_default();
                table
                    .cues
                    .iter()
                    .filter(|cue| !owned.contains(&cue.name))
                    .map(|cue| {
                        let mode = if cue.mode.is_loop() { "loop" } else { "once" };
                        let bad = problems.iter().any(|p| problem_names(p, &cue.name));
                        let mark = if bad { "  <- problem" } else { "" };
                        format!(
                            "{:<26} {:<10} {mode:<4} {:+5.1} dB  priority {}{mark}",
                            cue.name,
                            cue.bus_name(),
                            cue.gain_db,
                            cue.priority
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            // **The crate's own verdict on its table.** A cue on a bus the mix does not have, or one whose pitch range
            // is inverted, was drawn here with the same four columns as a healthy cue — nothing marked it, and the
            // panel's whole job is to make a fault visible. `problems_in(&settings)` is the settings-aware report
            // (iteration 21): the example's table uses `Ambience` and `Music`, and `problems()` assumes the crate's
            // default four, which would report every cue on them as `UnknownBus`.
            Kind::Problems => {
                // **A bounded block.** The panel is a fixed column with no scroll, and the log is the last thing in it;
                // with six faults the block grew past the window and pushed the log — the section that says what played
                // and why — off the bottom. Measured on a 1280x720 capture. Three sentences plus a count is what fits
                // under the cue list, and the rest is named rather than hidden.
                const SHOWN: usize = 3;
                let problems = table.problems_in(&settings);
                match problems.len() {
                    0 => "none".to_owned(),
                    n => {
                        let mut text =
                            problems.iter().take(SHOWN).map(|p| p.to_string()).collect::<Vec<_>>().join("\n");
                        if n > SHOWN {
                            // **Plain ASCII, not an em dash.** The panel's font has no glyph for `—` and drew a box;
                            // the capture showed it as `and 3 more ▯ the log`. A debug view cannot afford a character
                            // the font may not have.
                            text.push_str(&format!("\nand {} more; the log below is what played", n - SHOWN));
                        }
                        text
                    }
                }
            }
            // **Seven, because the `Music` block costs a heading and a line.** `SirenLog::CAPACITY`'s doc says *"a panel
            // shows eight"*, and this column fits eight — until a block is added above the log. Measured from the
            // captures: the Music block is 68 px (heading 40 + line 28) against the 63 px three removed cue rows freed,
            // so eight lines leaves **1 px** of margin — a clipped glyph, the defect iteration 32 fixed. Seven restores
            // it. The crate's number describes a panel; this panel has one more block than that sentence knew about.
            Kind::Log => log.recent(7).map(Entry::line).collect::<Vec<_>>().join("\n"),
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
        kept_at_s: f64::NEG_INFINITY,
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
        // **Sampling by real seconds, not by updates.** The layer fade is `MIX_TAU_S` (50 ms) — about six updates at
        // 120 fps — and sampling every twelve updates gave **0.5 samples per fade**: a GIF of the score's crossfade was a
        // coin flip, and the block's `-> target` arrow appeared or did not by the phase of the sample. A frame rate is
        // not a time, and the thing being captured is a fade.
        // `SIREN_SHOT_EVERY` is **hundredths of a second** (12 = 120 ms): a duration, which is what a fade is measured in.
        // It used to mean updates, and the two are not the same rate — see the doc on the knob.
        let every_s = c.every.max(1) as f64 / 100.0;
        let now_s = time.elapsed_secs_f64();
        if c.frame >= c.from && now_s - c.kept_at_s >= every_s {
            c.kept_at_s = now_s;
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
