//! Buses, ducking and the voice budget, asserted on [`SirenMix`] — the number a panel draws.

use bevy::prelude::*;
use bevy_siren::{
    BusLevel, Cue, CueTable, Duck, Entry, Play, Refusal, SirenLog, SirenMix, SirenSettings, SoundSlot, Voice,
};

use crate::helpers::{advance, app, live_voices, played, refused, run_for, sounds, voices};

/// A cue on `bus`, one variant, a given priority and polyphony.
fn cue(name: &str, bus: &str, priority: u8, max_instances: u16) -> Cue {
    Cue { sounds: vec![SoundSlot::new(name)], bus: bus.to_owned(), priority, max_instances, ..Cue::new(name) }
}

/// **A dialogue voice ducks the music by 8 dB, and the duck recovers after hold + fade.** This is the one duck
/// pair every game wants, and it is the default table.
#[test]
fn dialogue_ducks_the_music_and_recovers() {
    let mut app = app(
        CueTable::new(vec![cue("line", "Dialogue", 200, 1), cue("theme", "Music", 200, 1)]),
        SirenSettings::default(),
        sounds(&["line", "theme"]),
        false,
    );
    // The music at level, then the line.
    app.world_mut().write_message(Play::cue("theme"));
    app.update();
    app.update();
    assert!(app.world().resource::<SirenMix>().bus_db("Music") > -0.5, "the music is not ducked yet");

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    let mix = app.world().resource::<SirenMix>().clone();
    let ducked = mix.ducked.get("Music").copied().expect("a duck row for Music");
    assert!(
        (ducked - -8.0).abs() < 1e-5,
        "the first frame of a dialogue line ducks the music 8 dB: {ducked}"
    );
    assert!(
        mix.bus_db("Music") < -0.05,
        "the music's own level starts down the same frame: {}",
        mix.bus_db("Music")
    );
    assert_eq!(mix.voices_on("Dialogue"), 1);
    assert!(
        app.world()
            .resource::<SirenLog>()
            .iter()
            .any(|e| matches!(e, Entry::Ducked { bus, .. } if bus == "Music"))
    );

    // The music reaches the ducked level within a few time constants, and the duck recovers after its hold and
    // fade: 0.25 s of hold and 0.6 s of fade, so a mix a full second later is not ducked.
    run_for(&mut app, 0.2, 0.016);
    let ducked = app.world().resource::<SirenMix>().bus_db("Music");
    assert!((ducked - -8.0).abs() < 0.2, "the music is at its ducked level inside the hold: {ducked}");
    run_for(&mut app, 1.0, 0.016);
    let mix = app.world().resource::<SirenMix>().clone();
    assert!(mix.ducked["Music"] > -0.01, "a duck recovers after its hold and fade: {}", mix.ducked["Music"]);
}

/// **A bus capped at two voices steals the oldest for a third, and a request with nothing left to steal is
/// refused with `VoiceLimit` and spawns nothing.**
#[test]
fn a_full_bus_steals_the_oldest_and_then_refuses() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Music"), BusLevel::new("Effects").max_voices(2)],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![cue("hit", "Effects", 0, 4), cue("stinger", "Effects", 200, 1)]);
    let mut app = app(table, settings, sounds(&["hit", "stinger"]), false);

    // Two hits fill the bus.
    for _ in 0..2 {
        app.world_mut().write_message(Play::cue("hit"));
        app.update();
        advance(&mut app, 0.01);
    }
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).filter(|v| !v.unstealable).count(),
        2,
        "two voices on a bus capped at two"
    );

    // The third steals the oldest — the first `hit` — and plays.
    app.world_mut().write_message(Play::cue("hit"));
    app.update();
    let stopped = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .any(|e| matches!(e, Entry::Stopped { why, .. } if why.starts_with("stolen by")));
    assert!(stopped, "the oldest voice is stopped, and the log says why");

    // A high-priority voice on the bus is unstealable, so a request that needs it is refused.
    app.world_mut().resource_mut::<Time<Real>>().advance_by(std::time::Duration::from_millis(10));
    app.world_mut().write_message(Play::cue("stinger"));
    app.update();
    assert!(played(&mut app).iter().any(|p| p.cue == "stinger"), "the stinger plays");

    // Now the bus holds the stinger plus one, and nothing else may be stolen: the next request is refused.
    app.world_mut().resource_mut::<Time<Real>>().advance_by(std::time::Duration::from_millis(10));
    app.world_mut().write_message(Play::cue("stinger"));
    app.update();
    assert!(
        refused(&mut app).iter().any(|r| matches!(r.reason, Refusal::VoiceLimit { .. })),
        "a stinger's own cap is one, so a second is refused"
    );
}

/// **The table's own budget refuses a request rather than exceed it**, when every voice is unstealable.
#[test]
fn the_table_budget_refuses_rather_than_grow() {
    let settings = SirenSettings {
        max_voices: 1,
        buses: vec![BusLevel::new("Effects").max_voices(0)],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![cue("line", "Effects", 200, 0)]);
    let mut app = app(table, settings, sounds(&["line"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    assert_eq!(live_voices(&mut app), 1);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    assert!(refused(&mut app).iter().any(|r| matches!(r.reason, Refusal::VoiceLimit { .. })));
    assert_eq!(voices(&mut app), 1, "no 2nd voice past a budget of 1");
}

/// A cue's own `max_instances` is honoured: a second `Once` cue of one plays only after the first has ended, and a
/// looping one of one cannot be doubled.
#[test]
fn a_cue_respects_its_own_polyphony() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue { mode: bevy_siren::Mode::Loop, ..cue("hum", "Effects", 0, 1) }]);
    let mut app = app(table, settings, sounds(&["hum"]), false);

    app.world_mut().write_message(Play::cue("hum"));
    app.update();
    app.world_mut().write_message(Play::cue("hum"));
    app.update();
    assert_eq!(voices(&mut app), 1, "a looping cue of one voice cannot be doubled");

    // The log says the first was stolen to make room for the second, which is what a one-voice loop does.
    let stolen = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why.starts_with("stolen by hum")))
        .count();
    assert_eq!(stolen, 1, "{:?}", app.world().resource::<SirenLog>().iter().collect::<Vec<_>>());
}

/// A muted bus is silence, and unmuting returns its level — the mute is not a level written to zero.
#[test]
fn a_muted_bus_is_silent_and_keeps_its_level() {
    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![BusLevel::new("Music").gain_db(-3.0).muted()],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let mut app =
        app(CueTable::new(vec![cue("theme", "Music", 200, 1)]), settings, sounds(&["theme"]), false);
    app.world_mut().write_message(Play::cue("theme"));
    app.update();
    app.update();
    assert!(app.world().resource::<SirenMix>().bus_db("Music") <= bevy_siren::model::SILENT_DB + 0.5);

    app.world_mut().resource_mut::<SirenSettings>().buses[0].mute = false;
    run_for(&mut app, 0.8, 0.016);
    let db = app.world().resource::<SirenMix>().bus_db("Music");
    assert!((db - -3.0).abs() < 0.2, "unmuting restores the bus's own level: {db}");
}

/// The mix table's own mistakes are sentences: an unknown duck target, a bus listed twice, a budget of zero.
#[test]
fn problems_names_each_mistake() {
    let table = CueTable::new(vec![]);
    let empty = SirenSettings { max_voices: 0, buses: vec![], ducks: vec![], ..SirenSettings::default() };
    assert!(empty.problems().iter().any(|p| p.contains("max_voices is 0")));
    assert!(empty.bus_names().len() == 4, "a table with no buses is the crate's four");

    let doubled = SirenSettings {
        buses: vec![BusLevel::new("Music"), BusLevel::new("Music")],
        ducks: vec![
            Duck::new("Nope", "Music"),
            Duck::new("Music", "Nope"),
            Duck::new("Music", "Music").by_db(3.0),
        ],
        ..SirenSettings::default()
    };
    let problems = doubled.problems();
    assert!(problems.iter().any(|p| p.contains("listed more than once")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("starts on bus `Nope`")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("lowers bus `Nope`")), "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("raises it")), "{problems:?}");
    assert!(SirenSettings::default().problems().is_empty(), "{:?}", SirenSettings::default().problems());
    let _ = table;
}
