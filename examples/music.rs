//! A two-layer score following a scripted calm → combat → calm, so a bar change can be seen and timed.
//!
//! ```sh
//! cargo run --example music
//! ```
//!
//! It prints each transition's moment and the bar it landed on, then exits. At 100 bpm and four beats a bar, every
//! transition is a multiple of 2.4 s: a line that is not is a defect in `next_boundary`, and the example is the
//! check a person can read.

use bevy::asset::AssetPlugin;
use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy_siren::{
    Cue, CueTable, Mode, MusicLayer, MusicLayers, MusicPlugin, MusicScore, MusicState, SirenPlugin,
    SirenSettings, SirenSounds, SoundSlot,
};

/// The scripted fight: how long calm lasts, how long combat lasts, and the whole run.
const CALM_S: f64 = 3.0;
const COMBAT_S: f64 = 4.0;
const RUN_S: f64 = 10.0;

/// The score's tempo. A bar is 2.4 s, which is what every printed transition is checked against.
const BPM: f32 = 100.0;
const BAR_S: f64 = 2.4;

#[derive(Resource)]
struct Script {
    elapsed: f64,
    stage: Stage,
    printed: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Calm,
    Combat,
    CalmAgain,
    Done,
}

fn main() {
    App::new()
        // A headless run: no window and no device, but the asset store a sample is synthesised into.
        .add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .add_plugins((SirenPlugin, MusicPlugin))
        .insert_resource(SirenSettings::default())
        .insert_resource(cues())
        .insert_resource(score())
        .insert_resource(Script { elapsed: 0.0, stage: Stage::Calm, printed: Vec::new() })
        .add_systems(
            Startup,
            (setup, |mut layers: MessageWriter<MusicLayers>| {
                layers.write(MusicLayers::new(vec![("calm".to_owned(), -3.0)]).now());
            }),
        )
        .add_systems(Update, (script, report))
        .run();
}

fn cues() -> CueTable {
    CueTable::new(vec![
        Cue {
            sounds: vec![SoundSlot::new("calm")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("track.calm")
        },
        Cue {
            sounds: vec![SoundSlot::new("combat")],
            bus: "Music".to_owned(),
            mode: Mode::Loop,
            priority: 200,
            ..Cue::new("track.combat")
        },
    ])
}

fn score() -> MusicScore {
    MusicScore {
        layers: vec![MusicLayer::new("calm", "track.calm"), MusicLayer::new("combat", "track.combat")],
        tempo_bpm: BPM,
        beats_per_bar: 4.0,
    }
}

fn setup(mut sounds: ResMut<SirenSounds>, mut assets: ResMut<Assets<AudioSource>>) {
    for (key, hz) in [("calm", 196.0f32), ("combat", 233.0)] {
        let bytes = wav_bytes(hz, 3_000);
        sounds.insert(key, assets.add(AudioSource { bytes: bytes.into() }));
    }
}

/// A 16-bit mono 22.05 kHz WAV of a sine with a slow vibrato, so the two layers are audibly different tones.
fn wav_bytes(hz: f32, ms: u32) -> Vec<u8> {
    const RATE: u32 = 22_050;
    let count = (RATE as f32 * (ms as f32 / 1000.0)) as usize;
    let data_len = (count * 2) as u32;
    let mut out = Vec::with_capacity(44 + count * 2);
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
        let vibrato = 1.0 + 0.004 * (t * 5.0).sin();
        let sample = (t * hz * vibrato * std::f32::consts::TAU).sin() * 0.3;
        out.extend_from_slice(&((sample * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    out
}

/// The scripted fight: calm, then combat on the next bar, then calm again on the next bar after that.
fn script(
    mut script: ResMut<Script>,
    mut layers: MessageWriter<MusicLayers>,
    time: Res<Time<Real>>,
    mut exit: MessageWriter<AppExit>,
) {
    script.elapsed += time.delta_secs_f64();
    let elapsed = script.elapsed;
    match script.stage {
        Stage::Calm if elapsed >= CALM_S => {
            layers.write(
                MusicLayers::new(vec![
                    ("combat".to_owned(), -3.0),
                    ("calm".to_owned(), bevy_siren::model::SILENT_DB),
                ])
                .next_bar(),
            );
            script.stage = Stage::Combat;
        }
        Stage::Combat if elapsed >= CALM_S + COMBAT_S => {
            layers.write(
                MusicLayers::new(vec![
                    ("calm".to_owned(), -3.0),
                    ("combat".to_owned(), bevy_siren::model::SILENT_DB),
                ])
                .next_bar(),
            );
            script.stage = Stage::CalmAgain;
        }
        Stage::CalmAgain if elapsed >= RUN_S => {
            script.stage = Stage::Done;
            let bar = BAR_S;
            // Every transition the script asked for must have happened, or the check would pass vacuously.
            assert!(script.printed.len() >= 3, "the run printed only {} transitions", script.printed.len());
            let off = script
                .printed
                .iter()
                .filter_map(|l| l.split_whitespace().nth(2)?.parse::<f64>().ok())
                .any(|at| (at / bar - (at / bar).round()).abs() > 0.05);
            println!("music: transitions on the bar: {}", if off { "NO" } else { "yes" });
            exit.write(AppExit::Success);
        }
        _ => {}
    }
}

/// Prints each transition once, on the frame a layer's *target* moves — which is the frame the bar arrived on,
/// before the gain has begun to ease.
fn report(
    state: Res<MusicState>,
    time: Res<Time<Real>>,
    mut seen: Local<Option<(f32, f32)>>,
    mut script: ResMut<Script>,
) {
    let now = time.elapsed_secs_f64();
    let combat = state.target_db("combat").unwrap_or(bevy_siren::model::SILENT_DB);
    let calm = state.target_db("calm").unwrap_or(bevy_siren::model::SILENT_DB);
    let Some(previous) = *seen else {
        // The first frame is the baseline: the score starts silent and `Startup` moves calm in.
        *seen = Some((combat, calm));
        return;
    };
    if (combat - previous.0).abs() > 0.001 || (calm - previous.1).abs() > 0.001 {
        let line = format!("t = {now:.2} s  combat {combat:+.1} dB  calm {calm:+.1} dB");
        println!("{line}");
        script.printed.push(line);
        *seen = Some((combat, calm));
    }
}
