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
//! - `SIREN_KEYS=steal,layer,duck,refuse` stages one step every `SIREN_KEY_EVERY` frames (default 20) from frame 30,
//!   cycling: `duck` plays a line of dialogue, `steal` fills the bus capped at two, `layer` brings the combat layer in
//!   on the next bar, and `refuse` asks for a cue the table does not have — the log's `refused` line, which is what the
//!   panel is for.
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
    Attenuation, BusLevel, Cue, CueTable, Duck, Entry, Mode, MusicLayer, MusicLayers, MusicPlugin,
    MusicScore, MusicState, Play, SirenLog, SirenMix, SirenPlugin, SirenSettings, SirenSounds, SoundSlot,
    Stinger,
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
    /// **A play the crate answers with nothing.** The panel's claim is *"the log of what played and why"*, and a
    /// `Refused` is the "why" that matters most — a cue that never plays and never says why is the hardest thing in a
    /// game to find. Nothing else in this script refuses: `steal` steals, `duck` ducks, `layer` moves, and every one of
    /// them plays something. So the panel had never shown the one line a person opens it for.
    Refuse,
    /// **One cue's own polyphony, exercised.** `steal` demonstrates a *bus* cap; this demonstrates a *cue* cap — the
    /// number the cue list's `x2` names. Three `footstep.grass` requests in one frame against `max_instances: 2` leave
    /// two voices and one `stolen by footstep.grass` line, which is the row's `x2` made true.
    Poly,
    /// **A placed sound at a distance.** The only step that passes `.at(..)`, so the only one whose gain comes from
    /// `Attenuation` rather than the table: at 8 m the default `Inverse { near_m: 4, rolloff: 1 }` halves the amplitude,
    /// which is −6.02 dB. Also the only step that can produce `SpatiallyMute`.
    Spatial,
    /// **A sound whose owner was despawned.** `Play::on(entity)` parents the voice, so despawning the entity is the
    /// ordinary end of a sound on a vehicle or a torch — and until iteration 43 neither backend said so in the log. The
    /// step spawns an owner, hangs `engine.idle` from it, and drops the owner a step later.
    Parent,
    /// **A stinger: one hit on the `Music` bus that ducks what it plays over.** The table has had `music.stinger` since
    /// the example was written and nothing ever played it, so the panel had never shown the one duck whose cause is not a
    /// cue — the `Music` row's `duck -6.0 dB by stinger`, where `by` names the mechanism rather than a cue.
    Stinger,
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
    // **`SIREN_TIGHT=1` gives the `Music` bus one voice, so the second layer becomes a virtual voice.** The default
    // table has room for both layers, so the `waiting` marker would never appear and its one line could not be proved
    // by looking. The cap is on the **bus**, not the table: the other buses keep their room, so the picture isolates the
    // waiting layer instead of refusing every play in the script.
    let tight = std::env::var("SIREN_TIGHT").is_ok_and(|v| v != "0");
    // **`SIREN_MIX=broken` gives the *mix* table faults instead of the cue table.** `SIREN_FAULTS` adds cues that cannot
    // play; this adds a table that cannot mix — a master that is not a number, a bus listed twice, and a duck whose hold
    // is negative. Until iteration 45 the `Problems` block drew only the cue table's faults, so this table read
    // `none`: the report's other half was never on screen.
    let broken = std::env::var("SIREN_MIX").is_ok_and(|v| v == "broken");
    // **`SIREN_MIX=overbudget` gives the bus-cap fault its own table.** `broken` already fills the block's three-sentence
    // cap, so a fourth fault would be the invisible `and 1 more` — a capture that cannot show a sentence does not prove
    // it. This one has a single fault, so the whole block is that sentence.
    let overbudget = std::env::var("SIREN_MIX").is_ok_and(|v| v == "overbudget");
    // **`SIREN_MIX=zero` gives the zero-depth duck its own table**, for the same reason `overbudget` has one: `broken`
    // fills the three-sentence cap, so a fault added to it would be the invisible `and 1 more`. One rule at `by_db: 0.0`
    // with a real hold and fade — a duck that fires, is logged, and lowers the bus by nothing at all.
    let zero = std::env::var("SIREN_MIX").is_ok_and(|v| v == "zero");
    SirenSettings {
        master_db: if broken { f32::NAN } else { 0.0 },
        max_voices: if overbudget { 4 } else { 12 },
        buses: vec![
            // **Levels chosen so the demonstration has headroom.** `worst_peak_db` counts every live voice at full
            // scale, so two voices at 0 dB on a 0 dB bus reach **+6.02 dBFS** — measured at `+6.0` and `+4.0` on the
            // panel before this change, for `Effects` and `Music`. `2·10^(−7/20)` is **−0.98 dBFS** and at −7.1 it is
            // **−1.08**: both buses sit a decibel inside the rail, which is the "decibel in hand" the guide's headroom
            // section asks for. Nothing about the crate changed; the example stopped shipping over the rail.
            BusLevel::new("Music").gain_db(-7.1).max_voices(if tight { 1 } else { 0 }),
            // The bus asks for more than the table allows, so `bus_voice_limit` gives it the table's 4 and says so.
            BusLevel::new("Effects").gain_db(-7.0).max_voices(if overbudget { 16 } else { 2 }),
            BusLevel::new("Ambience").gain_db(-6.0),
            BusLevel::new("Dialogue"),
        ],
        ducks: if zero {
            vec![Duck::new("Dialogue", "Music").by_db(0.0).hold_s(0.25).fade_s(0.6)]
        } else if broken {
            // **Three faults, chosen so the block's three-sentence cap shows the two this loop just added.** A fourth
            // (a negative hold) pushed the self-duck off the bottom, and a capture that cannot see the sentence does not
            // prove it.
            vec![
                // no hold and no fade, on a rule whose source and target differ — the tenth sentence, alone
                Duck::new("Dialogue", "Music").by_db(-4.0),
                // both: no hold and no fade, *and* it starts on the bus it lowers — the eleventh
                Duck::new("Effects", "Effects").by_db(-6.0),
            ]
        } else {
            SirenSettings::default().ducks
        },
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
/// **How many sentences the `Problems` block draws**: three, or two and `and N more`. Measured on a 1280×720 capture —
/// the block is a fixed column with no scroll and the log is the last thing in it, so the bound is what keeps the log
/// on screen. The **log's** line count follows it, and both read this one number.
const PROBLEMS_SHOWN: usize = 3;

/// **How many cue rows the list draws.** Measured on a 1280×720 capture: eight rows fit beside a three-line `Problems`
/// block and six log lines. A longer table says `and N more` rather than pushing the log off the bottom — the block
/// under it already did this; iteration 45 gave the cue list the same rule after `SIREN_FAULTS=many`'s twelve rows
/// removed the log entirely.
const CUES_SHOWN: usize = 8;

/// **Where the `spatial` step puts `bang`, in metres.** 8 m is twice the default `Inverse` near radius, where the law
/// gives exactly half the amplitude — the number `cues.yaml` case 14 asserts (−6.02 dB). One constant, read by both the
/// step and the row, so the picture and the play cannot disagree.
const PLACED_M: f32 = 8.0;

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
        // **A placed sound.** The crate's spatial path — three attenuation curves, a panner on Bevy's backend, and a
        // `SpatiallyMute` refusal when no listener exists — was tested but never *drawn*: no cue in this table was
        // spatial and no step passed a position, so the panel could not show a distance or the gain one gives.
        Cue {
            sounds: vec![SoundSlot::new("hit_01")],
            bus: "Effects".to_owned(),
            priority: 30,
            spatial: true,
            ..Cue::new("bang")
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
            Kind::Header => {
                // **The header counts what the panel shows.** It said *"{} cues"* from `table.cues.len()`, and since the
                // `Music` block took the score's layers out of the cue list, that number and the rows beneath it disagreed:
                // a header saying **9** over a list of **7** reads as a lie even when the two moved cues are visible in
                // the `Music` block. Name the split instead.
                let layers = music.score.as_ref().map_or(0, |s| s.layers.len());
                let cues = table.cues.len().saturating_sub(layers);
                format!(
                    "bevy_siren: {cues} cues, {layers} layers, {} buses, {} voices",
                    settings.buses.len(),
                    mix.voices.values().sum::<usize>()
                )
            }
            Kind::BusesHeading
            | Kind::CuesHeading
            | Kind::MusicHeading
            | Kind::LogHeading
            | Kind::ProblemsHeading => continue,
            Kind::Bus(bus) => {
                let db = mix.bus_db(bus);
                let duck = mix.ducked.get(bus).copied().unwrap_or(0.0);
                // **A duck says how far *and* by what — from the same entry.** The row printed the **deepest** duck's
                // depth (`SirenMix::ducked`, the minimum over every active rule) beside the **newest** `Ducked` entry's
                // cause. When two ducks overlap — a stinger firing over a dialogue line, the ordinary case — it read
                // `duck -8.0 dB by stinger` where -8.0 belonged to `dialogue.greeting`: two numbers from different
                // sources presented as one fact. Iteration 31 chose the newest; this is the first iteration to put two
                // ducks on one bus and see it. Now the newest entry names the cause and **its own** depth, and when the
                // minimum comes from a different rule the row says which.
                let duck = if duck < -0.05 {
                    // **The newest entry names the cause; say why the depths differ, when they do.** Three cases:
                    //   1. one rule at its full depth — `duck -4.0 dB by <cue>`;
                    //   2. one rule **recovering**, so its `by_db` is deeper than the live depth — that is still the
                    //      cause and no second rule exists, so name it plainly;
                    //   3. two rules overlapping — the deepest need not be the newest, so both numbers need a label.
                    // Cases 1 and 2 are told apart from 3 by asking whether a *different* cause also names this bus,
                    // and whether the live depth's value appears among the recent entries at all.
                    let mut newest: Option<(&str, f32)> = None;
                    let mut other_cause = false;
                    for entry in log.recent(SirenLog::CAPACITY) {
                        if let Entry::Ducked { bus: b, by, by_db, .. } = entry
                            && b == bus
                        {
                            match newest {
                                None => newest = Some((by.as_str(), *by_db)),
                                Some((seen, _)) => other_cause |= seen != by.as_str(),
                            }
                        }
                    }
                    match newest {
                        // One live cause — whether at full depth or recovering — or several where the newest **is**
                        // still the deepest: either way one rule explains the number, so name it and stop.
                        Some((by, by_db)) if !other_cause || (by_db - duck).abs() < 0.05 => {
                            format!("   duck {duck:+.1} dB by {by}")
                        }
                        // Two causes: the deepest need not be the newest, so name the newest and its own depth.
                        Some((by, by_db)) => {
                            format!("   duck {duck:+.1} dB (newest: {by} at {by_db:+.1})")
                        }
                        None => format!("   duck {duck:+.1} dB"),
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
                        // **A layer the budget could not give a voice is a *virtual voice*.** `MusicState` records the
                        // live count it was refused at and retries when that count falls, and the panel drew
                        // `-80.0 dB` for it exactly as for a layer authored silent — two answers, one string. Gregory
                        // §14.5.8: the game believes it plays; only the panel could tell you otherwise.
                        let waiting = if music.blocked(name).is_some() { " waiting" } else { "" };
                        if (db - target).abs() >= 0.01 {
                            format!("{name} {db:+.1} -> {target:+.1}{waiting}")
                        } else {
                            format!("{name} {db:+.1} dB{waiting}")
                        }
                    })
                    .collect();
                // **A bus whose live voices can sum past full scale says so here, not in `Problems`.** `problems()`
                // describes the *table*, which does not change; a peak depends on this frame's voices, so it belongs with
                // the mix. `worst_peak_db` is an upper bound — each voice at full scale, summed in linear, times the bus's
                // gain (`docs/research/siren.md` R6: *"several sounds sum in linear"*). Nothing in `max_voices` prevents
                // it: that budget is a sample cost, and the example's own `Effects` reaches +6.0 dBFS with two voices.
                let hot: Vec<String> = mix
                    .voices
                    .iter()
                    .filter(|(_, n)| **n > 0)
                    .filter_map(|(bus, n)| {
                        let peak = mix.worst_peak_db(bus);
                        // **`>= 0.0`, not `> 0.0`**: a bus at exactly full scale has no headroom at all, and one voice at
                        // unity is exactly 0 dBFS — the case the shipped table is in most frames.
                        (peak >= 0.0).then(|| format!("{bus} {peak:+.1} dBFS / {n}"))
                    })
                    .collect();
                let headroom =
                    if hot.is_empty() { String::new() } else { format!("   peak {}", hot.join("  ")) };
                if parts.is_empty() {
                    format!("none{headroom}")
                } else {
                    format!("{}{headroom}", parts.join("   "))
                }
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
                let rows: Vec<&Cue> = table.cues.iter().filter(|cue| !owned.contains(&cue.name)).collect();
                // **The cue list is bounded the way the `Problems` block is**, because the panel is a fixed column with
                // no scroll. A table under `SIREN_FAULTS=many` has fourteen cues and drew twelve rows, which pushed the
                // `Problems` block and then the **whole log** off the bottom — measured, the last ink sat at `y = 719`
                // of 719 and the log's heading was not on screen at all. Eight rows is what fits beside a three-line
                // problems block and six log lines.
                let hidden = rows.len().saturating_sub(CUES_SHOWN);
                let mut lines: Vec<String> = rows
                    .iter()
                    .take(CUES_SHOWN)
                    .map(|cue| {
                        let mode = if cue.mode.is_loop() { "loop" } else { "once" };
                        let bad = problems.iter().any(|p| problem_names(p, &cue.name));
                        let mark = if bad { "  <- problem" } else { "" };
                        // **A placed cue says what the distance did to it.** The row shows the *authored* gain, which
                        // for a spatial cue is not what a player hears: the distance law multiplies it. The panel's
                        // spatial story — three attenuation curves, a panner, and `SpatiallyMute` — was tested and never
                        // drawn, because no cue in this table was spatial. This column is the one place a reader looks
                        // for "why is that one quiet", so it carries the law and the gain at the step's distance.
                        let placed = if cue.spatial {
                            let d = PLACED_M;
                            let g = bevy_siren::model::distance_gain(cue.attenuation, d);
                            format!(
                                "  {} at {d:.0} m {:+6.2} dB",
                                match cue.attenuation {
                                    Attenuation::None => "none".to_owned(),
                                    Attenuation::Inverse { .. } => "inverse".to_owned(),
                                    Attenuation::Linear { .. } => "linear".to_owned(),
                                },
                                bevy_siren::model::gain_to_db(g)
                            )
                        } else {
                            String::new()
                        };
                        // **How many of this cue may sound at once, which the row never said.** `priority` decides who
                        // survives a full budget; `max_instances` decides whether the *next* play may start at all,
                        // and it is the field that separates a footstep (two may overlap) from a click (one). A reader
                        // looking at `priority 10` had no way to know `footstep.grass` admits two — and `x` reads as a
                        // count where a bare `2` in the priority column would read as another priority.
                        let poly = if cue.max_instances == 0 {
                            // `max_instances: 0` is "no per-cue limit", and `problems()` already reports it as
                            // `UnlimitedInstances` — the row says so rather than inventing a number.
                            "all".to_owned()
                        } else {
                            format!("x{}", cue.max_instances)
                        };
                        format!(
                            "{:<26} {:<10} {mode:<4} {:+5.1} dB  priority {}  {poly:>3}{placed}{mark}",
                            cue.name,
                            cue.bus_name(),
                            cue.gain_db,
                            cue.priority
                        )
                    })
                    .collect();
                if hidden > 0 {
                    lines.push(format!("and {hidden} more; the log below is what played"));
                }
                lines.join("\n")
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
                // **The mix table's own faults, which this block never drew.** It read `table.problems_in(&settings)` —
                // the *cue* table's — so a mix table with `max_voices: 0`, a bus listed twice or a NaN master showed
                // `Problems: none` while every play was refused. `SirenSettings::problems()` is the other half of the
                // report, and the two are asked separately because they answer questions about different files.
                let mut problems: Vec<String> = settings.problems();
                problems.extend(table.problems_in(&settings).iter().map(ToString::to_string));
                match problems.len() {
                    0 => "none".to_owned(),
                    n => {
                        let mut text = problems
                            .iter()
                            .take(PROBLEMS_SHOWN)
                            .map(|p| p.to_string())
                            .collect::<Vec<_>>()
                            .join("\n");
                        if n > PROBLEMS_SHOWN {
                            // **Plain ASCII, not an em dash.** The panel's font has no glyph for `—` and drew a box;
                            // the capture showed it as `and 3 more ▯ the log`. A debug view cannot afford a character
                            // the font may not have.
                            text.push_str(&format!(
                                "\nand {} more; the log below is what played",
                                n - PROBLEMS_SHOWN
                            ));
                        }
                        text
                    }
                }
            }
            // **Six, because the panel grew a placed cue.** `SirenLog::CAPACITY`'s doc says *"a panel shows eight"*, and
            // this column fit seven until iteration 41 added `bang` — one cue row, 21 px. Measured on the capture with
            // the eighth row: the lowest ink sat at `y = 719` of 719, **0 px** of margin, the last line clipped. Six
            // restores it (measured 11 px). Each row costs a log line, so the count follows the table's height; the
            // alternative — dropping the spatial cue — would hide the one placed sound in the whole panel.
            // **Fewer lines when the `Problems` block has something in it.** Measured, the fault table costs THREE log
            // lines: it adds two cue rows *and* two problem sentences. Seven leaves `1 px` of margin (a clipped glyph),
            // five still leaves `1 px`, and **four restores 11 px**. The block's height is a function of the table it
            // describes, so the log's is too — a panel with faults shows fewer log lines, which is the right trade: the
            // faults are what the reader came for.
            // **The log's line count is what is left over, counted from the rows above it.** The panel is a fixed
            // column with no scroll, so every row a fault adds comes out of the log — the section that says what played
            // and why. Two rules were each right for one table and wrong for the other: "empty or not" (iteration 32)
            // could not count, and "the problems block's own height" (this iteration's first draft) ignored the six
            // extra **cue rows** `SIREN_FAULTS=many` adds, which measured **0 px** of margin — clipped. The rows are
            // counted where they are drawn: the cue list's, the problems block's, and the music block's one.
            Kind::Log => {
                let faults = settings.problems().len() + table.problems_in(&settings).len();
                let drawn = if faults == 0 { 1 } else { faults.min(PROBLEMS_SHOWN + 1) };
                // **Two blocks above the log can grow.** The `Problems` block draws one line for a clean table and one
                // per fault up to its bound; the cue list draws `CUES_SHOWN` rows **plus** an `and N more` line when it
                // truncates. Both come out of the log, and counting only the first left the log overflowing by
                // exactly that one row on `SIREN_FAULTS=many` — measured, a clipped glyph at `y = 719` of 719.
                let layers = music.score.as_ref().map_or(0, |s| s.layers.len());
                let cue_rows = table.cues.len().saturating_sub(layers);
                let overflow = drawn.saturating_sub(1) + usize::from(cue_rows > CUES_SHOWN);
                log.recent(6usize.saturating_sub(overflow)).map(Entry::line).collect::<Vec<_>>().join("\n")
            }
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
    mut stingers: MessageWriter<Stinger>,
    mut owners: Local<Vec<(Entity, u32)>>,
    mut commands: Commands,
) {
    let Some(c) = capture else {
        return;
    };
    // **The despawn follows its play by two frames**, long enough for the voice to start and short enough to share a
    // still with it. It runs every frame, *before* the step gate below, because the frame that must drop the owner is
    // usually not one on which the step fires.
    owners.retain(|(entity, at)| {
        if c.frame.saturating_sub(*at) < 2 {
            return true;
        }
        commands.entity(*entity).despawn();
        false
    });
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
        Step::Poly => {
            // Three in one frame against `max_instances: 2`. The third takes the first back, so two voices stay live and
            // the log says which went — the row's `x2`, made true. `Effects` holds two voices, so the first two also take
            // what was there; that is the documented rule (a cap is a budget on **plays**, not a standing count) and the
            // log shows both caps working on one line each, not one cap hiding the other.
            for _ in 0..3 {
                plays.write(Play::cue("footstep.grass"));
            }
        }
        Step::Parent => {
            // **A sound hung from an entity, then the entity dropped.** The frame this step runs spawns the owner and
            // plays on it; the next frame drops the owner, which is the end the log could not describe until iteration
            // 43. A fresh owner each time, so re-running the step is a second despawn and not a no-op.
            let owner = commands.spawn((Transform::default(), GlobalTransform::default())).id();
            owners.push((owner, c.frame));
            plays.write(Play::cue("engine.idle").on(owner));
        }
        Step::Spatial => {
            // 8 m: twice the default near radius, where the inverse law gives exactly half the amplitude.
            plays.write(Play::cue("bang").at(Vec3::new(0.0, 0.0, PLACED_M)));
        }
        Step::Stinger => {
            // `Stinger::new` defaults to -6 dB for a second, which is the crate's documented default and what the
            // `Music` row will show as `duck -6.0 dB by stinger`.
            stingers.write(Stinger::new("music.stinger"));
        }
        Step::Refuse => {
            // A cue the table does not have: `Refusal::UnknownCue`, named in the log and in `Refused`. The clearest
            // refusal there is — the reason is the cue's own name.
            plays.write(Play::cue("no.such.cue"));
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
        // **The default script includes `refuse`**, so the showcase shows the panel doing the thing it exists for. The
        // order ends on `refuse` before the still: a refusal is a line, not a sound, and it is the last thing a person
        // reading the log wants to have missed.
        .unwrap_or_else(|_| "steal,spatial,parent,layer,duck,stinger,poly,refuse".to_owned())
        .split(',')
        .filter_map(|token| match token.trim() {
            "duck" => Some(Step::Duck),
            "steal" => Some(Step::Steal),
            "layer" => Some(Step::Layer),
            "refuse" => Some(Step::Refuse),
            "stinger" => Some(Step::Stinger),
            "spatial" => Some(Step::Spatial),
            "poly" => Some(Step::Poly),
            "parent" => Some(Step::Parent),
            _ => None,
        })
        .collect();
    let keys = if keys.is_empty() {
        // **Louder than a silent fallback:** a token nobody recognises is a typo, and a capture that quietly staged
        // something else would put the wrong picture in the evidence folder.
        if std::env::var("SIREN_KEYS").is_ok_and(|v| !v.trim().is_empty()) {
            eprintln!(
                "siren: SIREN_KEYS has no step this example knows; staging the default steal,spatial,parent,layer,duck,stinger,poly,refuse"
            );
        }
        vec![
            Step::Steal,
            Step::Spatial,
            Step::Parent,
            Step::Layer,
            Step::Duck,
            Step::Stinger,
            Step::Poly,
            Step::Refuse,
        ]
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
