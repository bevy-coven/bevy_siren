//! One request, one answer: a headless app with no audio device still sees the decision.
//!
//! No `AudioPlugin` is added here, so nothing opens an output device — that is the point. Every assertion is on
//! [`Played`], [`Refused`] and the spawned [`Voice`], which is everything a play decides.

use bevy::prelude::*;
use bevy_siren::{Cue, CueTable, Mode, Play, Refusal, SirenSettings, SoundSlot, Voice};

use crate::helpers::{app, played, refused, run_for, sounds, voice_cues, voices};

/// A headless app with no audio device and the crate's default mix.
fn app_with(table: CueTable, sounds: bevy_siren::SirenSounds) -> App {
    app(table, SirenSettings::default(), sounds, false)
}

fn two_variant_step() -> CueTable {
    CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
        ..Cue::new("footstep.grass")
    }])
}

/// A cue with two variants plays once, spawns one `Voice` with a finite gain, and writes exactly one `Played`.
#[test]
fn a_play_becomes_one_voice_and_one_played() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01", "step_02"]));
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let answers = played(&mut app);
    assert_eq!(answers.len(), 1, "one request is one answer");
    let answer = &answers[0];
    assert_eq!(answer.cue, "footstep.grass");
    assert_eq!(answer.bus, "Effects");
    assert!(answer.sound == "step_01" || answer.sound == "step_02", "{}", answer.sound);
    assert!(answer.gain_db.is_finite(), "gain {}", answer.gain_db);
    assert!((-1.0..=1.0).contains(&answer.pitch), "pitch {}", answer.pitch);

    assert_eq!(voice_cues(&mut app), vec!["footstep.grass".to_owned()], "one entity carries the voice");
    let live = app.world_mut().query::<&Voice>().iter(app.world()).next().cloned().expect("the voice");
    assert!(!live.mode.is_loop());

    assert!(refused(&mut app).is_empty(), "a play is played or refused, never both");
}

/// The same cue asked twice in a row never plays the same variant: that is what `AvoidRepeat { avoid: 1 }` is
/// for, and it is the default.
#[test]
fn the_same_cue_twice_never_repeats_a_variant() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01", "step_02"]));
    let mut picks = Vec::new();
    for _ in 0..6 {
        app.world_mut().write_message(Play::cue("footstep.grass"));
        app.update();
        picks.extend(played(&mut app).into_iter().map(|p| p.sound));
    }
    assert_eq!(picks.len(), 6);
    for pair in picks.windows(2) {
        assert_ne!(pair[0], pair[1], "{picks:?}");
    }
}

/// An unknown cue is refused with that reason, and nothing is spawned: a refusal is not a quiet success.
#[test]
fn an_unknown_cue_is_refused_and_spawns_nothing() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01", "step_02"]));
    app.world_mut().write_message(Play::cue("footstep.snow"));
    app.update();

    let answers = refused(&mut app);
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].reason, Refusal::UnknownCue);
    assert_eq!(answers[0].cue, "footstep.snow");
    assert_eq!(voices(&mut app), 0);
    assert!(played(&mut app).is_empty());
}

/// A cue whose variant names a key with no handle is refused, and the reason names the key.
#[test]
fn a_missing_sound_names_the_key() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01"]));
    let mut reason = None;
    for _ in 0..4 {
        app.world_mut().write_message(Play::cue("footstep.grass"));
        app.update();
        if let Some(r) = refused(&mut app).first() {
            reason = Some(r.reason.clone());
            break;
        }
    }
    match reason.expect("the second variant has no handle, so one of four plays is refused") {
        Refusal::MissingSound { sound } => assert_eq!(sound, "step_02"),
        other => panic!("{other:?}"),
    }
}

/// A cue naming a bus the mix table does not have is refused with that bus, never silently played on `Effects`.
#[test]
fn an_unknown_bus_is_refused_by_name() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("wash")],
        bus: "Reverb".to_owned(),
        ..Cue::new("wash")
    }]);
    let mut app = app_with(table, sounds(&["wash"]));
    app.world_mut().write_message(Play::cue("wash"));
    app.update();
    assert_eq!(refused(&mut app)[0].reason, Refusal::UnknownBus { bus: "Reverb".to_owned() });
}

/// A spatial cue played with a position but no `SpatialListener` is refused rather than played at a guessed
/// position.
#[test]
fn a_spatial_play_with_no_ears_is_refused() {
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("bang")], spatial: true, ..Cue::new("bang") }]);
    let mut app = app_with(table, sounds(&["bang"]));
    let at = Vec3::new(3.0, 0.0, 4.0);
    app.world_mut().write_message(Play::cue("bang").at(at));
    app.update();
    assert_eq!(refused(&mut app)[0].reason, Refusal::SpatiallyMute(at));
}

/// A spatial cue heard from further away is quieter, and the gain follows the cue's attenuation curve.
#[test]
fn a_spatial_play_attenuates_with_distance() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("bang")],
        spatial: true,
        attenuation: bevy_siren::Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
        ..Cue::new("bang")
    }]);
    let mut app = app_with(table, sounds(&["bang"]));
    app.world_mut().spawn((Transform::default(), GlobalTransform::default(), SpatialListener::new(1.0)));

    app.world_mut().write_message(Play::cue("bang").at(Vec3::new(0.0, 0.0, 0.0)));
    app.update();
    let near = played(&mut app)[0].gain_db;
    assert!((near - 0.0).abs() < 1e-4, "at the near radius the gain is the cue's own: {near}");

    app.world_mut().write_message(Play::cue("bang").at(Vec3::new(0.0, 0.0, 8.0)));
    app.update();
    let far = played(&mut app)[0].gain_db;
    assert!((far - -6.0206).abs() < 1e-3, "twice the near radius is half the amplitude: {far}");
}

/// A request's own gain and pitch are added to the cue's, and both reach [`Played`].
#[test]
fn a_request_adjusts_gain_and_pitch() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("click")],
        gain_db: -2.0,
        pitch: bevy_siren::PitchRange::exact(0.0),
        ..Cue::new("click")
    }]);
    let mut app = app_with(table, sounds(&["click"]));
    app.world_mut().write_message(Play::cue("click").gain(-3.0).pitch(2.0));
    app.update();
    let answer = played(&mut app).remove(0);
    assert!((answer.gain_db - -5.0).abs() < 1e-5, "{}", answer.gain_db);
    assert!((answer.pitch - 2.0).abs() < 1e-5, "{}", answer.pitch);
}

/// A looping cue's voice survives a frame in which a one-shot's would have ended, because a loop has no end to
/// notice.
#[test]
fn a_loop_voice_lives_until_it_is_stopped() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("wind")],
        mode: Mode::Loop,
        bus: "Ambience".to_owned(),
        ..Cue::new("wind")
    }]);
    let mut app = app_with(table, sounds(&["wind"]));
    app.world_mut().write_message(Play::cue("wind"));
    for _ in 0..4 {
        app.update();
    }
    assert_eq!(voices(&mut app), 1);
    assert!(app.world_mut().query::<&Voice>().iter(app.world()).next().expect("the voice").mode.is_loop());
}

/// `StopCue` marks the voice and the log says why; the voice is gone after its fade.
#[test]
fn a_cue_can_be_stopped() {
    use bevy_siren::{SirenLog, StopCue};
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("wind")],
        mode: Mode::Loop,
        bus: "Ambience".to_owned(),
        ..Cue::new("wind")
    }]);
    let mut app = app_with(table, sounds(&["wind"]));
    app.world_mut().write_message(Play::cue("wind"));
    app.update();
    app.world_mut().write_message(StopCue { cue: "wind".to_owned() });
    run_for(&mut app, 0.8, 0.016);
    assert_eq!(voices(&mut app), 0, "the stopped voice is gone");
    let log = app.world().resource::<SirenLog>();
    assert!(
        log.iter().any(|e| matches!(e, bevy_siren::Entry::Stopped { why, .. } if why == "cue stopped")),
        "{:?}",
        log.iter().collect::<Vec<_>>()
    );
}

/// **A project's `seed` is the stream.** Two apps that differ only in `SirenSettings::seed` draw different variants
/// and different detunes; the same seed replays itself. This is `cues.yaml` case 17's own observation, and the
/// unfixed tree fails the `assert_ne!` — `SirenChoices` was built from a crate constant and never saw the settings.
#[test]
fn a_projects_seed_reaches_the_stream() {
    let table = || {
        CueTable::new(vec![Cue {
            sounds: vec![SoundSlot::new("a"), SoundSlot::new("b"), SoundSlot::new("c")],
            repeat: bevy_siren::Repeat::Shuffle,
            ..Cue::new("hit")
        }])
    };
    let run = |seed: u64| {
        let mut app =
            app(table(), SirenSettings { seed, ..SirenSettings::default() }, sounds(&["a", "b", "c"]), false);
        let mut out = Vec::new();
        for _ in 0..8 {
            app.world_mut().write_message(Play::cue("hit"));
            app.update();
            out.extend(played(&mut app).into_iter().map(|p| (p.sound, p.gain_db, p.pitch)));
        }
        out
    };

    let seven = run(7);
    let again = run(7);
    let ninetynine = run(99);

    assert_eq!(seven.len(), 8);
    assert_eq!(seven, again, "the same seed replays the same stream field for field");
    assert_ne!(seven, ninetynine, "two seeds are two streams, or the seed is not a seed");
}

/// **A seed change restarts the stream** rather than continuing from the old position: after the change, the next
/// four plays are a fresh seed-99 stream, not the old one carried on.
#[test]
fn changing_the_seed_restarts_the_stream() {
    use bevy_siren::SirenChoices;

    let table = || {
        CueTable::new(vec![Cue {
            sounds: vec![SoundSlot::new("a"), SoundSlot::new("b"), SoundSlot::new("c")],
            repeat: bevy_siren::Repeat::Shuffle,
            ..Cue::new("hit")
        }])
    };
    let play_four = |app: &mut App| {
        let mut out = Vec::new();
        for _ in 0..4 {
            app.world_mut().write_message(Play::cue("hit"));
            app.update();
            out.extend(played(app).into_iter().map(|p| (p.sound, p.pitch)));
        }
        out
    };

    let mut app = app_with(table(), sounds(&["a", "b", "c"]));
    let before = play_four(&mut app);
    assert_eq!(app.world().resource::<SirenChoices>().seed(), SirenSettings::default_seed());

    app.world_mut().resource_mut::<SirenSettings>().seed = 99;
    app.update();
    assert_eq!(app.world().resource::<SirenChoices>().seed(), 99, "the mix table's seed is the stream's");
    let after = play_four(&mut app);

    // A fresh app at seed 99, asked the same four times from its first play, is what "restarted" means.
    let mut fresh = crate::helpers::app(
        table(),
        SirenSettings { seed: 99, ..SirenSettings::default() },
        sounds(&["a", "b", "c"]),
        false,
    );
    let expected = play_four(&mut fresh);
    assert_eq!(
        after, expected,
        "after the change, the stream is a fresh seed-99 stream, not the old one continued"
    );
    assert_ne!(before, after, "the change is heard");
}
