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

/// **`MissingSound`'s sentence covers both of its causes** — a key with no handle, and bytes the decoder refused — so
/// a panel's line and the variant cannot drift. The key is what a project goes and looks at in both cases.
#[test]
fn the_missing_sound_sentence_covers_both_causes() {
    let line = Refusal::MissingSound { sound: "step_03".to_owned() }.to_string();
    assert_eq!(line, "no handle or undecodable bytes for sound `step_03`");
    assert!(line.contains("step_03"), "the file to look at is named");
}

/// **A non-spatial cue ignores a position.** `Cue::spatial` is "whether the cue is placed in the world"; a cue that
/// says no, played with `.at(..)`, must not be routed through the two-ear panner. The unfixed tree decided the panner
/// from `position.is_some()` alone, so such a voice played at rodio's `min(1, 1/d²)` — −38.7 dB at 8 m — while
/// `Played.gain_db` reported 0.00 dB and the log said it played. The sample and the meter disagreed.
/// The `PlaybackSettings` half of this test is Bevy's backend: with `--features mixer` the crate's own graph owns
/// playback and there is no settings component, so the entity shape is asserted only where that path is live. The
/// `Played.gain_db` assertion holds either way.
#[test]
fn a_non_spatial_cue_ignores_a_position() {
    use bevy::audio::AudioPlayer;
    use bevy::audio::PlaybackSettings;

    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("click")],
        spatial: false,
        ..Cue::new("click")
    }]);
    let mut app = app_with(table, sounds(&["click"]));
    app.world_mut().spawn((Transform::default(), GlobalTransform::default(), SpatialListener::new(1.0)));

    app.world_mut().write_message(Play::cue("click").at(Vec3::new(0.0, 0.0, 8.0)));
    app.update();

    let answer = played(&mut app).remove(0);
    assert!(
        (answer.gain_db - 0.0).abs() < 1e-4,
        "the cue's own gain, and true for the sample too: {}",
        answer.gain_db
    );

    let entity = answer.entity;
    if !cfg!(feature = "mixer") {
        let settings = app.world().get::<PlaybackSettings>(entity).expect("Bevy's path spawns with settings");
        assert!(!settings.spatial, "a non-spatial cue is not routed through the panner");
        assert!(app.world().get::<AudioPlayer>(entity).is_some(), "it still plays");
    }
    assert!(
        app.world().get::<Transform>(entity).is_none(),
        "no placed voice carries a position for a panner to read"
    );
}

/// **A spatial cue at a position is still panned, placed and compensated.** The case the fix must not overshoot: it
/// keeps its `Transform`, its panner flag, its distance gain and a non-zero `Compensation` for the panner's own law.
#[test]
fn a_spatial_cue_at_a_position_is_still_panned_and_compensated() {
    use bevy::audio::PlaybackSettings;
    use bevy_siren::Compensation;

    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("bang")],
        spatial: true,
        attenuation: bevy_siren::Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
        ..Cue::new("bang")
    }]);
    let mut app = app_with(table, sounds(&["bang"]));
    app.world_mut().spawn((Transform::default(), GlobalTransform::default(), SpatialListener::new(1.0)));

    app.world_mut().write_message(Play::cue("bang").at(Vec3::new(0.0, 0.0, 8.0)));
    app.update();

    let answer = played(&mut app).remove(0);
    assert!(
        (answer.gain_db - -6.0206).abs() < 1e-3,
        "twice the near radius halves the amplitude: {}",
        answer.gain_db
    );
    let entity = answer.entity;
    if !cfg!(feature = "mixer") {
        assert!(app.world().get::<PlaybackSettings>(entity).expect("settings").spatial);
    }
    let at = app.world().get::<Transform>(entity).expect("a placed voice reads a place").translation;
    assert!((at - Vec3::new(0.0, 0.0, 8.0)).length() < 1e-5);
    let compensation = app.world().get::<Compensation>(entity).expect("the panner's law is divided out");
    assert!(
        compensation.db > 1.0,
        "the panner would apply its own falloff; it is divided back out: {}",
        compensation.db
    );
}

/// **A spatial cue with no position is flat**, which is what its doc says: *"heard by everyone at the cue's own gain"*.
/// The fix must not start panning it.
#[test]
fn a_spatial_cue_without_a_position_is_still_flat() {
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("bang")], spatial: true, ..Cue::new("bang") }]);
    let mut app = app_with(table, sounds(&["bang"]));
    app.world_mut().spawn((Transform::default(), GlobalTransform::default(), SpatialListener::new(1.0)));

    app.world_mut().write_message(Play::cue("bang"));
    app.update();
    let answer = played(&mut app).remove(0);
    assert!((answer.gain_db - 0.0).abs() < 1e-4, "{}", answer.gain_db);
    assert!(app.world().get::<Transform>(answer.entity).is_none(), "no position, no placement");
}

/// **A spatial cue with no listener is still refused**, carrying the position — the crate's existing rule, unchanged.
#[test]
fn a_spatial_cue_with_no_listener_is_still_refused() {
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("bang")], spatial: true, ..Cue::new("bang") }]);
    let mut app = app_with(table, sounds(&["bang"]));
    let at = Vec3::new(3.0, 0.0, 4.0);
    app.world_mut().write_message(Play::cue("bang").at(at));
    app.update();
    assert_eq!(refused(&mut app)[0].reason, bevy_siren::Refusal::SpatiallyMute(at));
}

/// **A `StopVoice` for a voice that has already ended is a no-op, not a crash.** `retire_voices` despawns a one-shot the
/// frame it ends, and `Played::entity` is documented as the handle a game keeps for `StopVoice` — so holding it one
/// frame too long is the ordinary race between "the player pressed the button" and "the sound finished". The unfixed
/// `stop_requests` commanded a despawned entity, which panics Bevy's default error handler.
#[test]
fn a_stop_for_an_ended_voice_is_a_no_op() {
    use bevy_siren::StopVoice;

    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("a")], ..Cue::new("a") }]);
    let mut app = app_with(table, sounds(&["a"]));
    app.world_mut().write_message(Play::cue("a"));
    app.update();
    let entity = played(&mut app).remove(0).entity;

    // The voice ends and `retire_voices` despawns it — the crate's own path.
    app.world_mut().despawn(entity);
    app.update();
    assert!(app.world().get::<Voice>(entity).is_none(), "the voice is gone");

    // The game, holding the handle from `Played`, says stop. This must not panic.
    app.world_mut().write_message(StopVoice { entity });
    app.update();

    let log = app.world().resource::<bevy_siren::SirenLog>();
    assert!(
        !log.iter().any(|e| matches!(e, bevy_siren::Entry::Stopped { cue, .. } if cue == "a")),
        "there is nothing to stop, so nothing is logged: {:?}",
        log.iter().map(|e| e.line()).collect::<Vec<_>>()
    );
}

/// **A second `StopVoice` for a voice already fading is skipped** — a fade is begun once. The query already excludes
/// `Stopping`, and this pins the behaviour a project relies on when two systems both ask to stop the same sound.
#[test]
fn a_second_stop_for_a_fading_voice_is_skipped() {
    use bevy_siren::{Entry, SirenLog, StopVoice};

    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("wind")],
        mode: Mode::Loop,
        bus: "Ambience".to_owned(),
        ..Cue::new("wind")
    }]);
    let mut app = app_with(table, sounds(&["wind"]));
    app.world_mut().write_message(Play::cue("wind"));
    app.update();
    let entity = played(&mut app).remove(0).entity;

    app.world_mut().write_message(StopVoice { entity });
    app.update();
    app.world_mut().write_message(StopVoice { entity });
    app.update();

    let stopped = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why == "stop"))
        .count();
    assert_eq!(stopped, 1, "one fade, one line");
}

/// **`StopCue` still stops every voice of a cue** — the name path, unchanged by the guard, so the fix cannot narrow it.
#[test]
fn stop_cue_still_stops_every_voice_of_a_cue() {
    use bevy_siren::Entry;
    use bevy_siren::{SirenLog, StopCue};

    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("wind")],
        mode: Mode::Loop,
        bus: "Ambience".to_owned(),
        max_instances: 0,
        ..Cue::new("wind")
    }]);
    let mut app = app_with(table, sounds(&["wind"]));
    for _ in 0..2 {
        app.world_mut().write_message(Play::cue("wind"));
        app.update();
    }
    app.world_mut().write_message(StopCue { cue: "wind".to_owned() });
    app.update();

    let stopped = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why == "cue stopped"))
        .count();
    assert_eq!(stopped, 2, "both voices of the cue are stopped");
}

/// **`Backend(true)` with no `Output` must not give a voice nobody plays.**
///
/// `Backend` is a public resource whose flag a host may set, and `spawn_voice` decided "the mixer owns playback" from
/// that flag alone. With `Backend(true)` and no [`Output`], a voice carried **no `AudioPlayer`** (the mixer owns it) and
/// had **no mixer** (none exists) — it could not sound, could not end, and so could not be retired: it held its budget
/// slot for the life of the process while `Played` said it had played. `start_pending`'s doc already promises the
/// fallback (*"without an `Output` … Bevy's `AudioPlayer` owns every voice"*); the spawn never read the resource that
/// promise is about.
#[test]
fn a_backend_with_no_output_still_gives_the_voice_a_player() {
    let mut app = crate::helpers::app(
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("shot")], ..Cue::new("shot") }]),
        SirenSettings::default(),
        crate::helpers::sounds(&["shot"]),
        false,
    );
    // `SirenPlugin` inserts an `Output` when a device opened (the `mixer` feature does on this machine), so the state
    // under test is made explicit: `Backend(true)` and **no** `Output`, which is what a host gets by setting the flag.
    app.world_mut().remove_resource::<bevy_siren::output::Output>();
    app.insert_resource(bevy_siren::output::Backend(true));
    app.update();
    app.world_mut().resource_mut::<Messages<Play>>().write(Play::cue("shot"));
    app.update();

    let with_player = app.world_mut().query::<&bevy::audio::AudioPlayer>().iter(app.world()).count();
    assert_eq!(with_player, 1, "the voice has Bevy's player, so it can sound and end");
    assert_eq!(played(&mut app).len(), 1, "and the play is answered");
}

/// **The same cue with a live `Output` is owned by the mixer, and carries no `AudioPlayer`** — the two paths must not
/// both play one request.
///
/// This is the other half of the rule above: the fallback must not turn the mixer path into a double-play.
#[cfg(feature = "mixer")]
#[test]
fn a_backend_with_an_output_leaves_the_voice_to_the_mixer() {
    use bevy::prelude::*;
    use bevy_siren::output::{AudioOutput, Backend, Output};

    struct Silent;
    impl AudioOutput for Silent {
        fn set_bus_db(&mut self, _: &str, _: f32) {}
        fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}
        fn stop_voice(&mut self, _: &str, _: usize) {}
        fn play(&mut self, _: &str, _: &std::sync::Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
            Some(0)
        }
        fn finished(&self, _: &str, _: usize) -> bool {
            false
        }
    }

    let mut app = crate::helpers::app(
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("shot")], ..Cue::new("shot") }]),
        SirenSettings::default(),
        crate::helpers::sounds(&["shot"]),
        false,
    );
    app.insert_resource(Output(Box::new(Silent)));
    app.insert_resource(Backend(true));
    app.update();
    app.world_mut().resource_mut::<Messages<Play>>().write(Play::cue("shot"));
    app.update();

    let with_player = app.world_mut().query::<&bevy::audio::AudioPlayer>().iter(app.world()).count();
    assert_eq!(with_player, 0, "the mixer owns it, so Bevy must not also play it");
}

/// **A gain that is not a number is no gain, not silence.**
///
/// `chain_db` silences the whole chain on a non-finite part — right for a bus level the crate computed, wrong for a
/// number an author typed: the cue played **silent** while `Played` reported `-80.00` as though that level were
/// authored. R6 is explicit: *"a play is never silent: `Played` or `Refused`, both with the reason."* And the crate's
/// stated policy for the one other non-finite caller value it accepts puts the matter beyond doubt — `docs/guide/spatial.md`:
/// *"a caller handing in a `NaN` transform gets a sound at the listener's feet, not a `NaN` volume that silences the
/// whole bus."* One rule: a value that is not a number is the **neutral** one for what it measures.
#[test]
fn a_gain_that_is_not_a_number_is_no_gain() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("step_01")],
        gain_db: f32::NAN,
        ..Cue::new("footstep.grass")
    }]);
    let mut app = app_with(table, sounds(&["step_01"]));
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let answers = played(&mut app);
    assert_eq!(answers.len(), 1, "the play is answered, not dropped");
    assert_eq!(answers[0].gain_db, 0.0, "at the level the cue would have had without the NaN, not SILENT_DB");
    assert_eq!(voices(&mut app), 1, "and the voice exists");
}

/// **The same for a `Play`'s own gain override** — it enters by a different door and had the same silent result.
#[test]
fn a_plays_gain_that_is_not_a_number_is_no_gain() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01", "step_02"]));
    app.world_mut().write_message(Play::cue("footstep.grass").gain(f32::NEG_INFINITY));
    app.update();

    let answers = played(&mut app);
    assert_eq!(answers.len(), 1);
    assert_eq!(answers[0].gain_db, 0.0, "no gain applied, not silence");
}

/// **`Played.pitch` reports the rate the device is at**, not the number asked for.
///
/// `semitones_to_ratio` already played a non-finite detune at `1.0x` and clamped a large one to `16x`, but
/// `Started.pitch` carried the raw value into `Played`: the meter and the sound disagreed, which is the one thing
/// R6's second half (*"everything a play decides is readable"*) forbids. The clamp is four octaves each way, so
/// `+/-48` semitones is the number that actually plays.
#[test]
fn a_pitch_that_is_not_a_number_reports_the_speed_it_plays_at() {
    let mut app = app_with(two_variant_step(), sounds(&["step_01", "step_02"]));
    app.world_mut().write_message(Play::cue("footstep.grass").pitch(f32::NAN));
    app.world_mut().write_message(Play::cue("footstep.grass").pitch(1.0e9));
    app.update();

    let answers = played(&mut app);
    assert_eq!(answers.len(), 2);
    assert_eq!(answers[0].pitch, 0.0, "a NaN detune plays at 1.0x, so it reports 0 semitones");
    assert_eq!(answers[1].pitch, 48.0, "a detune past the clamp reports the clamp: four octaves each way");
}

/// **The guard: a silent bus still silences its voices.**
///
/// `chain_db`'s rule is what keeps a `NaN` **bus level** from making a game audible, and it must survive this fix. The
/// inputs are normalised; the model's own silence is untouched.
#[test]
fn a_silent_bus_still_silences_its_voices() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("step_01")],
        bus: "Effects".to_owned(),
        ..Cue::new("footstep.grass")
    }]);
    let settings = SirenSettings {
        buses: vec![bevy_siren::BusLevel::new("Effects").gain_db(bevy_siren::model::SILENT_DB)],
        ..SirenSettings::default()
    };
    let mut app = app(table, settings, sounds(&["step_01"]), false);
    app.world_mut().write_message(Play::cue("footstep.grass"));
    run_for(&mut app, 0.1, 1.0 / 60.0);

    let mix = app.world().resource::<bevy_siren::SirenMix>();
    assert!(
        mix.bus_db("Effects") <= bevy_siren::model::SILENT_DB,
        "the bus is silent: {}",
        mix.bus_db("Effects")
    );
}

/// **A duplicate key must not defeat `AvoidRepeat`.** `VariantPicker` refused the last **indices**, and a cue that lists
/// one file twice has two indices and one sound: under the crate's *default* `Repeat::AvoidRepeat { avoid: 1 }` such a
/// cue played the same recording **three times running** (`["a", "a", "b"]` ended `a a a`), which is the machine-gun
/// footstep `Repeat` exists to kill. A player hears keys, so the avoided window is made of keys.
#[test]
fn a_duplicate_key_never_repeats_under_avoid_repeat() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("a"), SoundSlot::new("a"), SoundSlot::new("b")],
        ..Cue::new("step")
    }]);
    let mut app = app_with(table, sounds(&["a", "b"]));
    let mut heard = Vec::new();
    for _ in 0..32 {
        app.world_mut().write_message(Play::cue("step"));
        app.update();
        heard.extend(played(&mut app).into_iter().map(|p| p.sound));
    }

    assert!(!heard.windows(2).any(|w| w[0] == w[1]), "no adjacent repeat: {heard:?}");
    assert!(
        heard.contains(&"a".to_owned()) && heard.contains(&"b".to_owned()),
        "and both sounds play: {heard:?}"
    );
}

/// **A cue of one sound still repeats.** The guard: a table with nothing to alternate has no alternative, and refusing
/// it would be the crate inventing a fault for a cue that is merely thin.
#[test]
fn a_cue_of_one_sound_still_repeats() {
    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("a")], ..Cue::new("shot") }]);
    let mut app = app_with(table, sounds(&["a"]));
    let mut heard = Vec::new();
    for _ in 0..4 {
        app.world_mut().write_message(Play::cue("shot"));
        app.update();
        heard.extend(played(&mut app).into_iter().map(|p| p.sound));
    }
    assert_eq!(heard, vec!["a".to_owned(); 4], "one sound is one sound: {heard:?}");
}

/// **A duplicate key still plays its own sound.** The fix must not move a play to a neighbour: `Played.sound` is the key
/// the cue's slot names, and `Played.variant` is a slot index into that cue's own list.
#[test]
fn a_duplicate_key_still_plays_its_own_sound() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("a"), SoundSlot::new("a"), SoundSlot::new("b")],
        ..Cue::new("step")
    }]);
    let mut app = app_with(table, sounds(&["a", "b"]));
    let mut heard = Vec::new();
    for _ in 0..8 {
        app.world_mut().write_message(Play::cue("step"));
        app.update();
        heard.extend(played(&mut app).into_iter().map(|p| p.sound));
    }
    // Every sound played is one this cue names, and the variant the voice reports is the slot that named it.
    let cues = app.world().resource::<CueTable>().clone();
    let cue = cues.get("step").expect("the cue");
    assert!(heard.iter().all(|s| cue.sounds.iter().any(|slot| &slot.sound == s)), "{heard:?}");
    let voices: Vec<String> =
        app.world_mut().query::<&Voice>().iter(app.world()).map(|v| v.cue.clone()).collect();
    assert!(voices.iter().all(|c| c == "step"), "and every voice is the cue asked for: {voices:?}");
}

/// **`pick` without keys is unchanged.** It is public, and a caller with no keys still gets index avoidance; the two
/// agree whenever the keys are distinct, which is every table that has no duplicate.
#[test]
fn pick_without_keys_still_avoids_indices() {
    use bevy_siren::model::{Rng, VariantPicker};
    let mut by_index = VariantPicker::new(1);
    let mut by_key = VariantPicker::new(1);
    let mut a = Rng::new(7);
    let mut b = Rng::new(7);
    let keys = ["x", "y", "z"];
    let mut index_picks = Vec::new();
    let mut key_picks = Vec::new();
    for _ in 0..16 {
        index_picks.push(by_index.pick(3, 1, &mut a).expect("a pick"));
        key_picks.push(by_key.pick_from(&keys, 1, &mut b).expect("a pick"));
    }
    assert_eq!(index_picks, key_picks, "distinct keys: the two rules agree");
    assert!(!index_picks.windows(2).any(|w| w[0] == w[1]), "{index_picks:?}");
}

/// **A sound on a despawned owner says it stopped, on both backends.** `Play::on` inserts `ChildOf(owner)`, so
/// despawning the owner despawns the voice — the ordinary end of a sound on a vehicle or a torch. Before this, neither
/// backend logged it: the crate's mixer freed the slot silently in `start_pending`, and Bevy's path had no entity left
/// for `retire_voices` to see. The log's last word about such a sound stayed `played` while the mix counted one fewer
/// voice, which is the one end path that had no line against `"ended"`, `"stop"`, `"cue stopped"` and `"stolen by …"`.
///
/// The line is written where the name can still be read — the frame the owner is gone and the voice is not — and
/// `Stopping` keeps a second system from writing it again.
#[test]
fn a_despawned_owner_stops_its_sound_and_says_so() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("engine")],
        mode: bevy_siren::Mode::Loop,
        ..Cue::new("engine")
    }]);
    let mut app = app_with(table, sounds(&["engine"]));
    // A parented voice needs a parent that can hold a child, and a `GlobalTransform` so it can be placed.
    let owner = app.world_mut().spawn((Transform::default(), GlobalTransform::default())).id();
    app.update();
    app.world_mut().write_message(Play::cue("engine").on(owner));
    app.update();
    assert_eq!(voices(&mut app), 1, "the layer is playing");

    app.world_mut().entity_mut(owner).despawn();
    app.update();

    let lines: Vec<String> =
        app.world().resource::<bevy_siren::SirenLog>().iter().map(|e| e.line()).collect();
    let said = lines.iter().filter(|l| l.contains("`engine`") && l.contains("owner despawned")).count();
    assert_eq!(said, 1, "exactly one line naming the sound: {lines:?}");
    assert_eq!(voices(&mut app), 0, "and the voice is gone with its owner");
}

/// **A cue whose `avoid` is raised after its first play gets the wider window on the next play.**
///
/// The picker for a cue is created on its **first** play and lives for the app's life. It used to keep that first
/// play's capacity, so a table the Cauldron reloads with a larger `Repeat::AvoidRepeat { avoid }` was silently inert:
/// four variants at `avoid: 3` with a picker built at `1` gave the byte-identical sequence to `avoid: 1`.
///
/// This is the end-to-end shape of that reload — one app, a cue whose `repeat` grows between two runs of plays.
#[test]
fn a_cue_whose_avoid_grows_gets_the_wider_window() {
    let cue = |avoid: u8| Cue {
        sounds: vec![SoundSlot::new("a"), SoundSlot::new("b"), SoundSlot::new("c"), SoundSlot::new("d")],
        repeat: bevy_siren::Repeat::AvoidRepeat { avoid },
        ..Cue::new("step")
    };
    // One app, `avoid: 1` for its first plays, then the wider rule.
    let mut app =
        app(CueTable::new(vec![cue(1)]), SirenSettings::default(), sounds(&["a", "b", "c", "d"]), false);
    let play = |app: &mut App| {
        app.world_mut().write_message(Play::cue("step"));
        app.update();
        played(app).into_iter().next().map(|p| p.sound)
    };
    let mut narrow = Vec::new();
    for _ in 0..8 {
        narrow.push(play(&mut app));
    }
    // The table now asks for a three-deep window; the picker was sized on the first play.
    *app.world_mut().resource_mut::<CueTable>() = CueTable::new(vec![cue(3)]);
    let mut wide = Vec::new();
    for _ in 0..12 {
        wide.push(play(&mut app));
    }
    // A three-deep window over four keys cannot put a key within three of itself: `adbcadbc`.
    for w in wide.windows(4) {
        assert!(
            w[0] != w[1] && w[0] != w[2] && w[0] != w[3],
            "at `avoid: 3` a key must not return within three picks: {wide:?}"
        );
    }
}
