//! **The README's Quick start, run — not compared.**
//!
//! The block a reader copies is `examples/quick_start.rs`. Until iteration 120 it was text in two places and a test that compared the two copies: it never played a sound, and the reader's own compiler said so
//! (`function walk is never used`) — `main` added neither `walk` nor `hear`, the keys had no extension so `load_table` asked for `audio/step_01`, and `hear` wrote a second, mistimed entry into the crate's own log
//! (`docs/loop/runs/2026-11-04-120/evidence/before/readme-as-written.txt`). Nothing here could fail because nothing here ran it.
//!
//! These tests bring the example in with `include!` — its functions are in scope, private as the reader's are — and run its **own** `setup`, `walk` and `report` on **the real asset server over `assets/`** (the directory
//! `DefaultPlugins` serves), **Bevy's real `AudioLoader`** and **the files a clone has**. They read **the console** — a `tracing` layer collecting each event's message, the way a person reads the terminal — and assert on
//! those lines. `Update` is forced single-threaded so a thread-local capture sees every system; the `Output` is removed so Bevy's path owns every voice in every build.
#![allow(dead_code)] // the example's `main` opens a window; here it is text that is included and never called.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::asset::{AssetPlugin, LoadState};
use bevy::audio::{AudioLoader, AudioSource};
use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::{Event, Subscriber};
use bevy::log::tracing_subscriber;
use bevy::log::tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use bevy::time::TimeUpdateStrategy;
use bevy_siren::output::Output;

use crate::helpers::voices;

// What the reader copies. Its `use` lines come with it.
include!("../examples/quick_start.rs");

const EXAMPLE: &str = include_str!("../examples/quick_start.rs");
const LIB: &str = include_str!("../src/lib.rs");

/// The `no_run` block in the crate-root doc, without its `//! ` prefixes or `# ` hidden lines: the quick start a reader meets on docs.rs.
fn crate_root_quick_start() -> Vec<&'static str> {
    let doc: Vec<&str> =
        LIB.lines().filter_map(|l| l.strip_prefix("//!")).map(|l| l.strip_prefix(' ').unwrap_or(l)).collect();
    doc.iter()
        .skip_while(|l| **l != "```no_run")
        .skip(1)
        .take_while(|l| **l != "```")
        .filter(|l| !(l.starts_with("# ") || **l == "#"))
        .copied()
        .collect()
}

/// What a game has that a headless test can have: the plugin `DefaultPlugins` adds for `assets/`, Bevy's own audio loader, the crate's plugin, and a clock the test drives.
fn console_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .init_asset_loader::<AudioLoader>()
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)))
        .add_plugins(SirenPlugin);
    app.world_mut().remove_resource::<Output>();
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}

/// Every event's target and message, in the order the console would have printed them.
#[derive(Clone, Default)]
struct Console(Arc<Mutex<Vec<(String, String)>>>);

impl<S: Subscriber> Layer<S> for Console {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        struct Message(String);
        impl Visit for Message {
            fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.0 = format!("{value:?}");
                }
            }
        }
        let mut message = Message(String::new());
        event.record(&mut message);
        self.0.lock().expect("the console").push((event.metadata().target().to_owned(), message.0));
    }
}

/// Runs `run` with the console captured **on this thread**, and returns what the quick start printed: the events whose target is this module, which is where `report`'s `info!` expands.
fn what_report_said(run: impl FnOnce()) -> Vec<String> {
    let console = Console::default();
    let subscriber = tracing_subscriber::registry().with(console.clone());
    bevy::log::tracing::subscriber::with_default(subscriber, run);
    let events = console.0.lock().expect("the console").clone();
    events.into_iter().filter(|(target, _)| target == module_path!()).map(|(_, message)| message).collect()
}

/// Updates a few milliseconds apart — the loader works on other threads — until every key the table named has loaded or failed, or five seconds have passed.
fn settle(app: &mut App) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        let sounds = app.world().resource::<SirenSounds>();
        let server = app.world().resource::<AssetServer>();
        let settled = !sounds.is_empty()
            && sounds.keys().all(|key| {
                let state = sounds.get(key).and_then(|handle| server.get_load_state(handle.id()));
                matches!(state, Some(LoadState::Loaded | LoadState::Failed(_)))
            });
        if settled {
            return;
        }
        assert!(Instant::now() < deadline, "the files were still loading after five seconds");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// The quick start's own wiring, which `main` writes: `setup` at startup, `walk` and `report` every frame.
fn wire(app: &mut App) {
    app.add_systems(Startup, setup).add_systems(Update, (walk, report));
}

/// The log's own lines, oldest first.
fn log_lines(app: &App) -> Vec<String> {
    app.world().resource::<SirenLog>().iter().map(Entry::line).collect()
}

/// **The block plays both recordings, one after the other, and the console says which — once each, in the crate's own words.**
///
/// Four seconds of 100 ms frames after the files have loaded is eight steps at the block's half-second cadence. The assertions are the reader's: the lines on the terminal are the crate's own log, in order, with none
/// twice (a prefix of it: the entries of the last frame belong to a system that is unordered against the crate's and may not have seen them), no `refused` among them, and the two files heard, never the same twice running.
#[test]
fn the_quick_start_plays_both_recordings_and_the_console_says_so() {
    let mut app = console_app();
    wire(&mut app);
    let said = what_report_said(|| {
        settle(&mut app);
        for _ in 0..40 {
            app.update();
        }
    });

    let log = app.world().resource::<SirenLog>();
    let plays: Vec<&str> = log
        .iter()
        .filter_map(|entry| match entry {
            Entry::Played { sound, .. } => Some(sound.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        plays.len() >= 6,
        "four seconds of footsteps at two a second: {plays:?}; the console said {said:?}"
    );
    assert!(
        plays.contains(&"step_01.ogg") && plays.contains(&"step_02.ogg"),
        "both recordings of the cue are heard: {plays:?}"
    );
    assert!(
        plays.windows(2).all(|pair| pair[0] != pair[1]),
        "never the same recording twice in a row: {plays:?}"
    );
    assert!(
        !log.iter().any(|entry| matches!(entry, Entry::Refused { .. })),
        "nothing is refused: {:?}",
        log_lines(&app)
    );

    let lines = log_lines(&app);
    assert!(!said.is_empty(), "the console showed the reader something");
    assert!(
        said.len() <= lines.len(),
        "the console printed {} lines for a log of {}: something was printed twice",
        said.len(),
        lines.len()
    );
    assert_eq!(said, lines[..said.len()], "the console is the crate's log, in order, each line once");
    assert!(
        lines.len() - said.len() <= 2,
        "and all of it but the last frame's entries: {} printed of {}",
        said.len(),
        lines.len()
    );
}

/// The old block's keys — `step_01`, with no extension — through the same `report`.
fn setup_with_the_old_keys(
    mut cue_table: ResMut<CueTable>,
    mut sounds: ResMut<SirenSounds>,
    assets: Res<AssetServer>,
) {
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

/// **The mistake the old block taught is named on the console.** The key `step_01` is not a file — the file is `step_01.ogg` — so `load_table` asked for `audio/step_01`, and the only sign a reader had was Bevy's own
/// `ERROR … Path not found`, which names neither this crate, a cue nor a key. Through `report` the line is the crate's: the cue, the key, **why**, and the path that was looked for.
#[test]
fn a_key_without_its_extension_is_named_on_the_console() {
    let mut app = console_app();
    app.add_systems(Startup, setup_with_the_old_keys).add_systems(Update, (walk, report));
    let said = what_report_said(|| {
        settle(&mut app);
        for _ in 0..40 {
            app.update();
        }
    });

    let refusal = said
        .iter()
        .find(|line| line.contains("refused `footstep.grass`"))
        .unwrap_or_else(|| panic!("a refusal was printed; the console said {said:?}"));
    assert!(
        refusal.contains("sound `step_0") && refusal.contains("failed to load"),
        "it names the key and says the load failed: {refusal}"
    );
    assert!(
        refusal.contains("assets/audio/step_0") && !refusal.contains(".ogg"),
        "and the path that was looked for, which has no extension: {refusal}"
    );
    assert_eq!(voices(&mut app), 0, "and nothing is left playing");
}

/// Every function `text` defines but its `main` does not name — what the compiler reported as `never used` and nobody read.
fn unscheduled(text: &str) -> Vec<&str> {
    let main = text.split("fn main() {").nth(1).and_then(|rest| rest.split("\n}").next()).unwrap_or("");
    let named: Vec<&str> = main.split(|c: char| !c.is_alphanumeric() && c != '_').collect();
    text.lines()
        .filter_map(|line| line.strip_prefix("fn "))
        .filter_map(|line| line.split('(').next())
        .filter(|name| *name != "main" && !named.contains(name))
        .collect()
}

/// **A block that defines a system its `main` does not add is wrong whatever it compiles to.** The first form is the old README's: `main` added `setup` and nothing else, and the block defined `walk` and `hear`.
#[test]
fn the_quick_start_adds_every_system_it_defines() {
    const NEVER_ADDED: &str = "fn main() {\n    App::new().add_systems(Startup, setup).run();\n}\n\nfn setup() {}\n\nfn walk() {}\n\nfn hear() {}\n";
    assert_eq!(unscheduled(NEVER_ADDED), ["walk", "hear"], "the check sees what the old block did");
    assert_eq!(
        unscheduled(EXAMPLE),
        Vec::<&str>::new(),
        "the quick start's `main` names every system it defines"
    );
}

/// **The crate-root doc's block is the example — in a plain `cargo test`.** The README's own comparison is behind `readme-example` (`tests/readme.rs`); this one is not, so the doctest a reader sees on docs.rs cannot
/// drift from the file that is run.
#[test]
fn the_crate_root_doc_is_the_example() {
    let doc = crate_root_quick_start();
    assert!(!doc.is_empty(), "src/lib.rs has a ```no_run block");
    assert_eq!(
        doc,
        EXAMPLE.lines().collect::<Vec<_>>(),
        "src/lib.rs's quick start differs from examples/quick_start.rs, the file the tests run"
    );
}
