//! **The tour: the whole crate, heard.** Nine scenes, one feature each, scripted so a person can listen and so a run can
//! be replayed.
//!
//! ```sh
//! cargo run --example demo
//! ```
//!
//! A window, a camera, a listener, and a script that walks through what the crate does. Every sound is a real file under
//! `assets/audio/demo`, made by `tools/make_demo_sounds.py`; the crate picks, mixes, ducks, layers, limits and places
//! them, and the panel reads `SirenMix`, `SirenLog` and `MusicState` — the resources a game's own meter reads.
//!
//! # The scenes
//!
//! | # | seconds | what it shows |
//! |---|---|---|
//! | 1 | 0 – 3 | three footsteps: one cue, three recordings, never the same one twice in a row |
//! | 2 | 3 – 5 | a burst: one sample, an exact pitch — a play with no decision in it |
//! | 3 | 5 – 9 | the calm music layer, looping on `Music`: phrases that decay to silence between notes |
//! | 4 | 9 – 13 | a line of dialogue ducks `Music` by 8 dB and `Effects` by 4, holds, and recovers |
//! | 5 | 13 – 17 | the combat layer comes in **on the bar** and the calm one goes quiet |
//! | 6 | 17 – 20 | a stinger arrives at once and ducks the music it plays over |
//! | 7 | 20 – 24 | six hits against a cue limit of three: three play, three are taken back |
//! | 8 | 24 – 28 | the same sound at four metres and at sixteen: distance decides the level |
//! | 9 | 28 – 32 | a cue name that does not exist: refused, and the console says why |
//!
//! Then it repeats from scene 3, so a listener who arrives late hears everything within half a minute.
//!
//! # Keys and knobs
//!
//! `Space` pauses the script (the mix keeps running — it is on real time), `R` restarts at scene 1, `Esc` quits.
//! `SIREN_SHOT=<path>` saves one still at `SIREN_FRAMES` and quits, which is how `docs/media/demo.png` is made;
//! `SIREN_SIZE=WxH` sets the window.
//!
//! # Why a panel as well as a console
//!
//! The console is the crate's: `report` prints every `SirenLog` entry once, in the crate's words. The panel exists
//! because a duck, a steal and a bar change are judged by watching them move, and because every number on it is the
//! crate's own rather than the demo's.

use bevy::audio::SpatialListener;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::text::FontSize;
use bevy::ui::widget::Text;
use bevy::window::WindowResolution;
use bevy_siren::{
    Attenuation, BusLevel, Cue, CueTable, Mode, MusicLayer, MusicLayers, MusicPlugin, MusicScore, MusicState,
    Play, Repeat, SirenChoices, SirenLog, SirenMix, SirenPlugin, SirenSettings, SirenSounds, SoundSlot,
    Stinger,
};

/// Where the demo's recordings live, relative to `assets/` (the directory `DefaultPlugins` serves).
const AUDIO: &str = "audio/demo";

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "bevy_siren: the tour".to_owned(),
            resolution: window_size(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins((SirenPlugin, MusicPlugin))
    .init_resource::<Tour>()
    .add_message::<Advanced>()
    .add_systems(Startup, (setup, build_panel))
    .add_systems(Update, (advance, emit, report, draw_panel, keys, capture));
    if std::env::var("SIREN_FRAMES").is_ok() {
        app.insert_resource(capture_config());
    }
    app.run();
}

/// The window a person gets and the one a capture saves: one function, so a still and a run cannot disagree about layout.
fn window_size() -> WindowResolution {
    match std::env::var("SIREN_SIZE")
        .ok()
        .and_then(|s| s.split_once('x').map(|(w, h)| (w.parse(), h.parse())))
    {
        Some((Ok(w), Ok(h))) => WindowResolution::new(w, h).with_scale_factor_override(1.0),
        _ => WindowResolution::new(1280, 720).with_scale_factor_override(1.0),
    }
}

/// **The cue table**: one cue per thing the tour shows, and nothing in it that `problems()` would name.
///
/// `tests/demo.rs` asserts both halves — that `problems()` is empty, and that every key it names has a file — so a
/// renamed recording is a red test rather than a silent demo.
pub fn table() -> CueTable {
    CueTable::new(vec![
        // Three recordings and the default window: never the same foot twice.
        Cue {
            sounds: vec![
                SoundSlot::new("step_01.ogg"),
                SoundSlot::new("step_02.ogg"),
                SoundSlot::new("step_03.ogg"),
            ],
            gain_db: -3.0,
            // **A walk has two feet.** `max_instances` defaults to one — the answer for a burst or a click — and with it
            // the second footstep in a scene *stole the first* (`stolen by footstep.grass (cue)`, heard in this
            // example's first real run at 1.11 s, where a recording 0.26 s long had not finished). Two is a walk.
            max_instances: 2,
            ..Cue::new("footstep.grass")
        },
        // One recording and an exact pitch: no decision in the play at all, which scene 2 is about.
        Cue {
            sounds: vec![SoundSlot::new("hit_01.ogg")],
            pitch: bevy_siren::PitchRange::exact(0.0),
            repeat: Repeat::Shuffle,
            gain_db: -4.0,
            ..Cue::new("gun.burst")
        },
        // **The A half of the variant vignette**: one recording, played eight times. The demo plays it only to show what
        // the crate's picker exists to prevent, and `tests/demo.rs` keeps it out of the windowed tour's script.
        Cue {
            sounds: vec![SoundSlot::new("step_01.ogg")],
            repeat: Repeat::Shuffle,
            gain_db: -3.0,
            max_instances: 2,
            ..Cue::new("footstep.single")
        },
        // A UI click: one sample, its own bus, and a cue that must not move any other cue's draws.
        Cue {
            sounds: vec![SoundSlot::new("click.ogg")],
            pitch: bevy_siren::PitchRange::exact(0.0),
            ..Cue::new("ui.click")
        },
        // **A bed the score does not own.** It lives on `Ambience`, not `Music`: `adopt_layers` tags every voice on the
        // `Music` bus as a music layer and drives it to the score's target, so a stem played directly on `Music` is
        // silenced (measured: the layer vignette's A half was −180 dBFS, and the duck's bed never played at all). A bed
        // on `Ambience` is what a game's ambience is anyway, and the duck's rule `Dialogue -> Ambience` exercises the
        // same code as `Dialogue -> Music`.
        Cue {
            sounds: vec![SoundSlot::new("music_calm.ogg")],
            mode: Mode::Loop,
            bus: "Ambience".to_owned(),
            gain_db: -4.0,
            ..Cue::new("bed")
        },
        // Two music stems, started silent by the score and brought in on a bar.
        Cue {
            sounds: vec![SoundSlot::new("music_calm.ogg")],
            mode: Mode::Loop,
            bus: "Music".to_owned(),
            ..Cue::new("music.calm")
        },
        Cue {
            sounds: vec![SoundSlot::new("music_combat.ogg")],
            mode: Mode::Loop,
            bus: "Music".to_owned(),
            ..Cue::new("music.combat")
        },
        // A stinger: a one-shot that is never stolen, played over the music and ducking it.
        Cue {
            sounds: vec![SoundSlot::new("stinger.ogg")],
            bus: "Music".to_owned(),
            priority: bevy_siren::UNSTEALABLE_PRIORITY,
            ..Cue::new("music.stinger")
        },
        // **A cue with no per-cue limit worth speaking of**, for the budget vignette: the *table's* `max_voices` is the cap
        // under test there, and `footstep.grass`'s own `max_instances: 2` binds first in both halves — which is why that
        // vignette's A and B came out sample-identical (0 bytes different) until this cue existed.
        Cue {
            sounds: vec![
                SoundSlot::new("step_01.ogg"),
                SoundSlot::new("step_02.ogg"),
                SoundSlot::new("step_03.ogg"),
            ],
            max_instances: 64,
            gain_db: -3.0,
            ..Cue::new("footstep.many")
        },
        // A one-shot with its own polyphony limit, which scene 7 fills.
        // **Six of these sum past full scale if each one is at unity**: measured in the first run of `demo_render`, six
        // hits at the same instant reached **+2.5 dBFS with 38 samples at the rail**. The crate mixes samples and never
        // bounds the sum (`SirenMix::worst_peak_db` is how a game asks), so a cue that can stack says what it is worth:
        // −9 dB each puts three at about −4.4 dBFS, which is a hit and not a clip.
        Cue {
            sounds: vec![SoundSlot::new("hit_01.ogg")],
            max_instances: 3,
            priority: 10,
            gain_db: -9.0,
            ..Cue::new("hit")
        },
        // Two lines of dialogue, on the bus that ducks the other two.
        Cue {
            sounds: vec![SoundSlot::new("line_01.ogg"), SoundSlot::new("line_02.ogg")],
            repeat: Repeat::AvoidRepeat { avoid: 1 },
            bus: "Dialogue".to_owned(),
            priority: bevy_siren::UNSTEALABLE_PRIORITY,
            // A line is one at a time, and the default says so; written out because a reader of this table should see
            // every field the demo depends on.
            max_instances: 1,
            ..Cue::new("dialogue.greeting")
        },
        // A spatial cue: the same recording at any distance, the law deciding the level.
        Cue {
            sounds: vec![SoundSlot::new("hit_01.ogg")],
            spatial: true,
            attenuation: Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
            // **Two at once, because the scene is a comparison**: at four metres and at sixteen, at the same instant.
            // At the default of one the second play stole the first, and a listener heard one distance, not two.
            max_instances: 2,
            ..Cue::new("bang")
        },
    ])
}

/// **The mix**: the crate's defaults, with `Effects` capped at three so scene 7's steal has a cap to hit.
///
/// `Ambience` is still one of the four buses even though nothing on it plays in this tour: the crate ships four, and a
/// demo that renamed the set would be demonstrating a different crate.
pub fn settings() -> SirenSettings {
    SirenSettings {
        buses: vec![
            BusLevel::new("Music").gain_db(-6.0).max_voices(4),
            BusLevel::new("Effects").gain_db(-3.0).max_voices(3),
            // **Room for the bed.** At the crate's default table budget the bed was *refused* — `refused \`music.calm\`:
            // no voice free` — because the score's two stems took the first slots before the vignette's own play arrived,
            // and the duck then lowered a bus with nothing playing on it. Measured in `tour.txt`, not guessed.
            BusLevel::new("Ambience").gain_db(-9.0).max_voices(4),
            BusLevel::new("Dialogue"),
        ],
        ..SirenSettings::default()
    }
}

/// **The score**: two stems of `Music` at the tempo the pads were written for, so a layer change on the bar is on the grid.
pub fn score() -> MusicScore {
    MusicScore {
        layers: vec![MusicLayer::new("calm", "music.calm"), MusicLayer::new("combat", "music.combat")],
        tempo_bpm: 100.0,
        beats_per_bar: 4.0,
    }
}

fn setup(
    mut commands: Commands,
    mut sounds: ResMut<SirenSounds>,
    mut table_res: ResMut<CueTable>,
    mut settings_res: ResMut<SirenSettings>,
    assets: Res<AssetServer>,
) {
    commands.spawn((Camera2d, SpatialListener::new(1.0)));
    // **A backdrop, so the still is a picture and not a black rectangle.** The capture came back 1280 × 720 of one
    // colour until this existed: a 2D camera with no sprite and no `ClearColor` draws the default clear, and a panel of
    // text on it is invisible in a PNG. The colour matches `examples/mixer`'s panel.
    commands.insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)));
    *table_res = table();
    *settings_res = settings();
    commands.insert_resource(score());
    // One call, and every key the table names is asked for as `<AUDIO>/<key>`.
    let asked = sounds.load_table(&assets, &table_res, AUDIO);
    info!("the tour: {} keys asked of `{AUDIO}/`", asked.len());
}

/// Where the script is.
#[derive(Resource, Default)]
struct Tour {
    /// Which scene, an index into [`SCENES`].
    scene: usize,
    /// How long this scene has been up, on real time.
    at_s: f32,
    /// A pause holds `at_s` and silences [`emit`]; the mix keeps running, because it is on real time.
    paused: bool,
    /// The scene's start has fired.
    started: bool,
    /// A scene's one-shot has fired (the burst, the six hits, the two distances).
    fired: bool,
}

/// What the script has done, for a test: written when a scene's start fires.
#[derive(Message, Clone, Debug, PartialEq)]
struct Advanced(usize);

/// What a scene does **at its start**, as data — one table a reader and a test both read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Start {
    /// Nothing but the footsteps [`emit`] writes.
    Footsteps,
    /// One burst, from [`emit`].
    Burst,
    /// The bed and the calm layer come up.
    Bed,
    /// One line of dialogue.
    Line,
    /// The combat layer in, on the next bar.
    LayerOnTheBar,
    /// One stinger.
    Stinger,
    /// Six hits, from [`emit`].
    Hits,
    /// Two distances, from [`emit`].
    Distances,
    /// A cue name the table does not have.
    Mistake,
}

/// The tour: nine scenes, each a title for the panel, a sentence for the panel, a length, and what its start does.
struct Scene {
    title: &'static str,
    what: &'static str,
    len_s: f32,
    start: Start,
}

const SCENES: [Scene; 9] = [
    Scene {
        title: "1 · three footsteps",
        what: "one cue, three recordings, never the same one twice",
        len_s: 3.0,
        start: Start::Footsteps,
    },
    Scene {
        title: "2 · a burst",
        what: "one sample, an exact pitch: a play with no decision in it",
        len_s: 2.0,
        start: Start::Burst,
    },
    Scene {
        title: "3 · a bed and a pad",
        what: "a looping ambience and the calm layer, each on its own bus",
        len_s: 4.0,
        start: Start::Bed,
    },
    Scene {
        title: "4 · a line ducks the music",
        what: "Dialogue lowers Music by 8 dB and Effects by 4",
        len_s: 4.0,
        start: Start::Line,
    },
    Scene {
        title: "5 · the layer changes on the bar",
        what: "combat comes in, calm goes quiet, on a bar of 100 bpm",
        len_s: 4.0,
        start: Start::LayerOnTheBar,
    },
    Scene {
        title: "6 · a stinger",
        what: "arrives at once and ducks the music it plays over",
        len_s: 3.0,
        start: Start::Stinger,
    },
    Scene {
        title: "7 · the budget",
        what: "six hits, a limit of three: three play, three are taken back",
        len_s: 4.0,
        start: Start::Hits,
    },
    Scene {
        title: "8 · distance",
        what: "the same sound at 4 m and at 16 m: the law decides",
        len_s: 4.0,
        start: Start::Distances,
    },
    Scene {
        title: "9 · a mistake",
        what: "a cue that does not exist: refused, and the console says which",
        len_s: 4.0,
        start: Start::Mistake,
    },
];

/// **The script.** Moves to the next scene when this one's time is up and fires its start once; the repeating emitters are
/// in [`emit`].
fn advance(mut tour: ResMut<Tour>, time: Res<Time<Real>>, mut advanced: MessageWriter<Advanced>) {
    if tour.paused {
        return;
    }
    if !tour.started {
        tour.started = true;
        tour.fired = false;
        advanced.write(Advanced(tour.scene));
    }
    tour.at_s += time.delta_secs();
    if tour.at_s >= SCENES[tour.scene].len_s {
        tour.scene = (tour.scene + 1) % SCENES.len();
        tour.at_s = 0.0;
        tour.started = false;
        tour.fired = false;
    }
}

/// What a scene's **start** writes. Kept apart from [`advance`] so the decisions are a table, not a system's branches.
fn start_of(
    scene: usize,
    play: &mut MessageWriter<Play>,
    music: &mut MessageWriter<MusicLayers>,
    sting: &mut MessageWriter<Stinger>,
) {
    match SCENES[scene].start {
        Start::Footsteps | Start::Burst | Start::Hits | Start::Distances => {}
        Start::Bed => {
            music.write(
                MusicLayers::new(vec![
                    ("calm".to_owned(), -3.0),
                    ("combat".to_owned(), bevy_siren::model::SILENT_DB),
                ])
                .now(),
            );
        }
        Start::Line => {
            play.write(Play::cue("dialogue.greeting"));
        }
        Start::LayerOnTheBar => {
            // **On the next bar**: the score's tempo decides when, not this system.
            music.write(MusicLayers::new(vec![
                ("combat".to_owned(), -3.0),
                ("calm".to_owned(), bevy_siren::model::SILENT_DB),
            ]));
        }
        Start::Stinger => {
            sting.write(Stinger::new("music.stinger"));
        }
        Start::Mistake => {
            play.write(Play::cue("footstep.grss"));
        }
    }
}

/// The per-frame emitters: a footstep twice a second in scene 1, and each one-shot scene's burst.
fn emit(
    mut tour: ResMut<Tour>,
    mut play: MessageWriter<Play>,
    mut music: MessageWriter<MusicLayers>,
    mut sting: MessageWriter<Stinger>,
    mut next_step_at_s: Local<f32>,
    mut last_scene: Local<Option<usize>>,
) {
    // **A scene change restarts the walk's clock.** `at_s` is the scene's own time, so a `Local` that outlived the scene
    // would either fire at once (a later visit to scene 1) or never (a clock already past the scene's length).
    if *last_scene != Some(tour.scene) {
        *last_scene = Some(tour.scene);
        *next_step_at_s = 0.0;
    }
    if tour.paused {
        return;
    }
    // The start's decisions, once, here rather than in `advance` so they have the message writers.
    if !tour.started {
        return;
    }
    if !tour.fired && !matches!(SCENES[tour.scene].start, Start::Footsteps | Start::Hits | Start::Distances) {
        start_of(tour.scene, &mut play, &mut music, &mut sting);
        tour.fired = true;
    }
    match tour.scene {
        // A footstep twice a second: a walk.
        0 => {
            if tour.at_s >= *next_step_at_s {
                play.write(Play::cue("footstep.grass"));
                // **Resync, do not catch up.** A frame that took longer than the cadence — the first frame after a
                // window opens does, measured here at 0.92 s — would otherwise fire a second step in that frame and the
                // walk would limp. The next step is due half a second from *now*, at most.
                *next_step_at_s = tour.at_s + 0.5;
            }
        }
        // Six hits in one frame, once the scene has been up a moment, so a listener hears the steal inside the scene.
        6 if !tour.fired && tour.at_s > 0.4 => {
            for _ in 0..6 {
                play.write(Play::cue("hit"));
            }
            toured(&mut tour);
        }
        // The same cue at four metres and at sixteen — one play each, and the law decides their levels.
        7 if !tour.fired && tour.at_s > 0.4 => {
            play.write(Play::cue("bang").at(Vec3::new(4.0, 0.0, 0.0)));
            play.write(Play::cue("bang").at(Vec3::new(16.0, 0.0, 0.0)));
            toured(&mut tour);
        }
        _ => {}
    }
}

/// Marks the scene's one-shot as fired. A named function because the borrow checker wants `&mut tour` inside a `match`
/// that already holds `tour`, and a one-line `fn` says what the line means.
fn toured(tour: &mut Tour) {
    tour.fired = true;
}

/// Prints the crate's log once each, in its words — the `report` the README's quick start has.
fn report(log: Res<SirenLog>, mut seen: Local<u64>) {
    for entry in log.since(*seen) {
        info!("{}", entry.line());
    }
    *seen = log.pushed();
}

/// One still at `SIREN_FRAMES` to `SIREN_SHOT`, then quit: the command `docs/media/README.md` gives, so a still cannot
/// drift from the program.
#[derive(Resource)]
struct Capture {
    at: u32,
    frame: u32,
    shot: String,
}

fn capture_config() -> Capture {
    let at = std::env::var("SIREN_FRAMES").ok().and_then(|n| n.parse().ok()).unwrap_or(420);
    let shot = std::env::var("SIREN_SHOT").unwrap_or_else(|_| "demo.png".to_owned());
    Capture { at, frame: 0, shot }
}

fn capture(capture: Option<ResMut<Capture>>, mut commands: Commands, mut exit: MessageWriter<AppExit>) {
    let Some(mut c) = capture else { return };
    c.frame += 1;
    if c.frame == c.at {
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(c.shot.clone()));
    }
    // **The screenshot is taken in `PostUpdate` and read back over the following frames**, so quitting too soon tears the
    // window down first: measured at `+1` frame — `Unknown window for screenshot, skipping` then
    // `Failed to send screenshot: sending on a closed channel`, and no file. `examples/mixer` quits at `+10` and its
    // captures work; the same margin is used here rather than a second guess.
    if c.frame == c.at + 10 {
        exit.write(AppExit::Success);
    }
}

/// `Space` pauses the script, `R` restarts it, `Esc` quits.
fn keys(keys: Res<ButtonInput<KeyCode>>, mut tour: ResMut<Tour>, mut exit: MessageWriter<AppExit>) {
    if keys.just_pressed(KeyCode::Space) {
        tour.paused = !tour.paused;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        *tour = Tour::default();
    }
    if keys.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// The one text node the panel draws into.
#[derive(Component)]
struct Panel;

fn build_panel(mut commands: Commands) {
    // A full-window panel background, then the text on it: one node each, so the layout is a rectangle a still shows.
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            padding: UiRect::all(Val::Px(20.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.06, 0.07, 0.09)),
    ));
    commands.spawn((
        Text::new(String::new()),
        TextFont { font_size: FontSize::Px(15.0), ..default() },
        TextColor(Color::srgb(0.86, 0.89, 0.93)),
        Node { position_type: PositionType::Absolute, left: Val::Px(20.0), top: Val::Px(16.0), ..default() },
        Panel,
    ));
}

/// The panel's text: the scene, the four buses, the two layers, and the last six decisions — every number the crate's.
#[allow(clippy::too_many_arguments)]
fn draw_panel(
    tour: Res<Tour>,
    mix: Res<SirenMix>,
    music: Res<MusicState>,
    sounds: Res<SirenSounds>,
    table_res: Res<CueTable>,
    choices: Res<SirenChoices>,
    log: Res<SirenLog>,
    mut panel: Query<&mut Text, With<Panel>>,
) {
    let Ok(mut text) = panel.single_mut() else { return };
    let scene = &SCENES[tour.scene];
    let mut lines = vec![
        format!("bevy_siren · the tour      scene {}", scene.title),
        format!("                            {}", scene.what),
        String::new(),
        "bus          level    duck   voices".to_owned(),
    ];
    for bus in ["Music", "Effects", "Ambience", "Dialogue"] {
        let duck = mix.ducked.get(bus).copied().unwrap_or(0.0);
        lines.push(format!(
            "{:<12} {:>6.1} dB  {:>5.1}   {}",
            bus,
            mix.bus_db(bus),
            duck,
            mix.voices_on(bus)
        ));
    }
    let layer = |name: &str| match music.layer_db(name) {
        Some(db) => format!("{db:>6.1} dB"),
        None => "     ---".to_owned(),
    };
    lines.push(String::new());
    lines.push(format!("music        calm {}   combat {}", layer("calm"), layer("combat")));
    let missing = sounds.missing(&table_res);
    lines.push(format!(
        "sounds       {} keys loaded{}   seed {}",
        sounds.keys().count(),
        if missing.is_empty() { String::new() } else { format!(", {} missing", missing.len()) },
        choices.seed()
    ));
    lines.push(String::new());
    lines.push("decisions, newest first".to_owned());
    for entry in log.recent(6) {
        lines.push(format!("  {}", entry.line()));
    }
    lines.push(String::new());
    lines.push("Space pause · R restart · Esc quit".to_owned());
    text.0 = lines.join("\n");
}
