//! **An authoring refusal is printed once, and a budget refusal never is.**
//!
//! `docs/guide/cues.md` promises *"There is no silent failure"*: every refused play is a `Refused` with a reason. But a
//! game that reads neither `Refused` nor `SirenLog` — the ordinary game — learned **nothing** from a misspelt cue name,
//! measured in `docs/loop/runs/2026-11-04-123/`: ten refusal classes, ten silent consoles, before iteration 123.
//!
//! The crate now prints an **authoring** refusal with `target: "bevy_siren"`, once per cue and reason per frame. This
//! file holds both directions: the four that print, the two that do not, and the rule that a hundred plays in one frame
//! are one mistake.
//!
//! The console is read the way `tests/quick_start.rs` reads it — a `tracing` layer collecting each event's message — and
//! `Update` is forced single-threaded, because `with_default` installs a subscriber on **this** thread.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::log::tracing::field::{Field, Visit};
use bevy::log::tracing::{Event, Subscriber};
use bevy::log::tracing_subscriber;
use bevy::log::tracing_subscriber::layer::{Context, Layer, SubscriberExt};
use bevy::prelude::*;
use bevy_siren::*;

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

/// What the crate printed on the console: the events whose target is `bevy_siren`, which is where its `warn!` expands.
fn crate_said(run: impl FnOnce()) -> Vec<String> {
    let console = Console::default();
    let subscriber = tracing_subscriber::registry().with(console.clone());
    bevy::log::tracing::subscriber::with_default(subscriber, run);
    console
        .0
        .lock()
        .expect("the console")
        .iter()
        .filter(|(target, _)| target == "bevy_siren")
        .map(|(_, message)| message.clone())
        .collect()
}

fn table() -> CueTable {
    CueTable::new(vec![
        Cue { sounds: vec![SoundSlot::new("a"), SoundSlot::new("b")], ..Cue::new("footstep.grass") },
        Cue { sounds: vec![SoundSlot::new("a")], bus: "Reverb".to_owned(), ..Cue::new("bang") },
        Cue { sounds: vec![SoundSlot::new("a")], max_instances: 2, ..Cue::new("engine.idle") },
        // Never stolen (`cues.yaml` case 9): a table of two voices refuses the third play rather than taking one back.
        Cue { sounds: vec![SoundSlot::new("a")], priority: UNSTEALABLE_PRIORITY, ..Cue::new("line") },
        Cue { sounds: Vec::new(), ..Cue::new("silent.cue") },
        Cue { sounds: vec![SoundSlot::new("a")], spatial: true, ..Cue::new("bang.spatial") },
    ])
}

/// A headless app with the shipped plugin, two handles and no device, `Update` on this thread so a thread-local
/// subscriber sees every event.
fn console_app(settings: SirenSettings) -> App {
    let mut sounds = SirenSounds::new();
    for key in ["a", "b"] {
        sounds.insert(key, Handle::<AudioSource>::default());
    }
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(table())
        .insert_resource(sounds)
        .insert_resource(settings)
        .add_plugins(SirenPlugin);
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app.update();
    app
}

/// `count` plays of `cue` in one frame, then the frame.
fn play_in_one_frame(app: &mut App, cue: &str, count: usize) {
    for _ in 0..count {
        app.world_mut().write_message(Play::cue(cue));
    }
    app.update();
}

/// **An authoring mistake is on the console, once per cue and reason per frame** — and a healthy game prints nothing.
///
/// Four refusals are a project's own: a cue name the table does not have, a cue with no sounds, a key with no handle, a
/// bus the mix table does not have. Each is printed with the cue and the log's sentence, once: a hundred plays of a
/// misspelt name in one frame are **one** typo.
#[test]
fn an_authoring_refusal_is_printed_once_per_frame_and_named() {
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        play_in_one_frame(&mut app, "footstep.grss", 100);
    });
    assert_eq!(said, ["refused `footstep.grss`: no cue of that name"], "a hundred plays are one line");

    // A second cue is a second mistake, in the same frame.
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        app.world_mut().write_message(Play::cue("footstep.grss"));
        app.world_mut().write_message(Play::cue("footstep.snow"));
        app.update();
    });
    assert_eq!(said.len(), 2, "two unknown cues are two lines: {said:?}");
    assert!(said[0].contains("footstep.grss") && said[1].contains("footstep.snow"), "{said:?}");

    // A second **frame** is a second line: the mistake is still there, and a console that went quiet would read fixed.
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        for _ in 0..3 {
            play_in_one_frame(&mut app, "footstep.grss", 1);
        }
    });
    assert_eq!(said.len(), 3, "one line a frame: {said:?}");

    // The other three authoring refusals, each in its own words.
    for (setup, play, want) in [
        ("a cue with no sounds", "silent.cue", "the cue has no sounds"),
        ("an unknown bus", "bang", "bus `Reverb` is not in the mix table"),
        ("a key with no handle", "footstep.grass", "no handle or undecodable bytes"),
    ] {
        let said = crate_said(|| {
            let mut app = console_app(SirenSettings::default());
            if setup == "a key with no handle" {
                // The cue names `a` and `b`; this app has neither.
                app.world_mut().insert_resource(SirenSounds::new());
            }
            play_in_one_frame(&mut app, play, 1);
        });
        assert_eq!(said.len(), 1, "{setup}: {said:?}");
        assert!(said[0].contains(want), "{setup}: {} does not say {want:?}", said[0]);
        assert!(
            said[0].starts_with(&format!("refused `{play}`: ")),
            "{setup}: it names the cue: {}",
            said[0]
        );
    }

    // **Two reasons for one cue in one frame are two lines**, not one: `MissingSound` (the handle) is decided before
    // `UnknownBus` (the mix table), so a cue with no handle **and** a bus the table does not have reads as
    // `MissingSound` — the first reason is the one a project has to fix first. What this holds is that the line is
    // keyed by the reason and not by the cue alone: the same cue refused for a reason, then a different one, is two.
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        app.world_mut().insert_resource(SirenSounds::new());
        play_in_one_frame(&mut app, "bang", 1);
    });
    assert_eq!(said.len(), 1, "one reason, one line: {said:?}");
    assert!(said[0].contains("no handle"), "and it is the first reason a project must fix: {said:?}");

    // **The refusal that must NOT print**: a healthy cue, eight plays, nothing on the console.
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        play_in_one_frame(&mut app, "footstep.grass", 8);
    });
    assert!(said.is_empty(), "a healthy game prints no refusal: {said:?}");
}

/// **A budget refusal is not a fault, and a spatial mute is not one either**: neither prints, however often it happens.
///
/// A `VoiceLimit` is a cap doing what a project asked for (`cues.yaml` cases 7–9 and 21); `SpatiallyMute` is an
/// architectural state — a scene whose listener arrives a frame after its first play would otherwise print at frame zero
/// of every run. Both are still `Refused` messages and `SirenLog` entries.
#[test]
fn a_budget_refusal_and_a_spatial_mute_are_not_printed() {
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings { max_voices: 2, ..SirenSettings::default() });
        play_in_one_frame(&mut app, "line", 40);
    });
    assert!(said.is_empty(), "the budget is normal operation: {said:?}");

    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        app.world_mut().write_message(Play::cue("bang.spatial").at(Vec3::new(1.0, 0.0, 0.0)));
        app.update();
    });
    assert!(said.is_empty(), "a spatial mute is an architectural state: {said:?}");

    // And the refusals are still answers: a game that reads `Refused` sees them.
    let mut app = console_app(SirenSettings { max_voices: 2, ..SirenSettings::default() });
    play_in_one_frame(&mut app, "line", 40);
    // The log keeps the refusal a message may have been drained of: the point is that the console said nothing.

    let log = app.world().resource::<SirenLog>();
    let refusals: Vec<String> = log
        .iter()
        .filter_map(|entry| match entry {
            Entry::Refused { cue, reason, .. } => Some(format!("{cue}: {reason}")),
            _ => None,
        })
        .collect();
    assert!(
        refusals.iter().any(|r| r.contains("no voice free")),
        "the budget refused, and the log says so: {refusals:?}"
    );
}

/// **The console and the log cannot disagree**: the printed line is the `Entry::Refused` reason, behind the cue's name.
#[test]
fn the_console_line_is_the_logs_own_sentence() {
    let mut lines = Vec::new();
    let said = crate_said(|| {
        let mut app = console_app(SirenSettings::default());
        play_in_one_frame(&mut app, "footstep.grss", 1);
        let log = app.world().resource::<SirenLog>();
        lines.extend(log.recent(1).map(Entry::line).collect::<Vec<_>>());
    });
    assert_eq!(said.len(), 1, "{said:?}");
    assert_eq!(lines.len(), 1, "{lines:?}");
    let reason = lines[0]
        .split_once("refused `footstep.grss`: ")
        .map(|(_, rest)| rest)
        .expect("the log's refusal line");
    assert_eq!(said[0], format!("refused `footstep.grss`: {reason}"), "one sentence, two places");
}

/// **An app with no subscriber prints nothing and does not panic** — every ordinary headless test, and a game that
/// removed Bevy's `LogPlugin`. A `tracing` event with no subscriber is dropped.
#[test]
fn no_subscriber_is_silent_and_safe() {
    let mut app = console_app(SirenSettings::default());
    play_in_one_frame(&mut app, "footstep.grss", 10);
    // And the same set of refusals was answered, once each.
    let refused: HashSet<String> =
        app.world_mut().resource_mut::<Messages<Refused>>().drain().map(|r| r.cue).collect();
    assert_eq!(refused, HashSet::from(["footstep.grss".to_owned()]), "ten plays, one cue, answered");
}
