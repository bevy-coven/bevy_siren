//! Buses, ducking and the voice budget, asserted on [`SirenMix`] — the number a panel draws.

use bevy::prelude::*;
use bevy_siren::{
    BusLevel, Cue, CueTable, Duck, Entry, Mode, Play, Refusal, SirenLog, SirenMix, SirenSettings, SoundSlot,
    StopCue, Voice,
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
    // **The sentence names the cap, not only the cue.** Here the *cue's* `max_instances` took the voice, so an author
    // who wanted two voices of `hum` must raise the cue's number — `(bus)` would send them to the wrong one.
    let why = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .find_map(|e| match e {
            Entry::Stopped { why, .. } if why.starts_with("stolen by hum") => Some(why.clone()),
            _ => None,
        })
        .expect("the steal was logged");
    assert!(why.ends_with("(cue)"), "a cue cap must say so: {why}");
}

/// **A steal names which cap took the voice, because the three caps want three different edits.** `max_instances`
/// (raise the cue's x2), the bus's `max_voices`, and the table's `max_voices` all produce `stolen by <cue>`, and before
/// this the sentence was identical for all three: a step that took three voices logged two lines saying the same thing
/// and neither said which number to change. This test forces the **bus** cap specifically and asserts the sentence says
/// `(bus)` — the mutation (one shared format string without `cap.as_str()`) reddens both this and
/// `a_cue_respects_its_own_polyphony`.
#[test]
fn a_steal_names_the_cap_that_took_the_voice() {
    let settings = SirenSettings {
        max_voices: 8,
        // The bus holds one voice while the cue's own `max_instances` is 4, so the **bus** is what refuses.
        buses: vec![BusLevel::new("Effects").max_voices(1)],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue { max_instances: 4, ..cue("hum", "Effects", 0, 1) }]);
    let mut app = app(table, settings, sounds(&["hum"]), false);

    app.world_mut().write_message(Play::cue("hum"));
    app.update();
    app.world_mut().write_message(Play::cue("hum"));
    app.update();

    let why = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .find_map(|e| match e {
            Entry::Stopped { why, .. } if why.starts_with("stolen by hum") => Some(why.clone()),
            _ => None,
        })
        .expect("the bus cap took the first voice");
    assert!(why.ends_with("(bus)"), "the bus is what refused, so the sentence must say so: {why}");
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

/// **The mix table's report names every mistake it knows, and the doc above it lists the same nine.**
///
/// `MusicScore::problems` has had this test since iteration 34, after its doc named four of its nine checks. The mix
/// table's doc named **six of its nine**, and four of the nine sentences had no test at all — measured by fixed-string
/// search over `tests/`: `master_db is not a number`, `a bus has no name`, `a bus ... has a gain that is not a number`,
/// and `a duck ... has a hold or fade that is negative`. A sentence nothing reaches is what an author trusts instead of
/// the sound, unread.
///
/// The mutation is any of the nine `out.push` lines in `src/mix.rs::problems`, which reddens the assertion naming it.
#[test]
fn problems_names_every_mistake() {
    let settings = |buses: Vec<BusLevel>, ducks: Vec<Duck>, master_db: f32, max_voices: u16| SirenSettings {
        master_db,
        max_voices,
        buses,
        ducks,
        ..SirenSettings::default()
    };
    let music = || BusLevel::new("Music");

    // One table, every fault it can hold. Faults that cannot coexist — a bus named and unnamed, `max_voices` zero and
    // not — are two tables, because one would hide the other's sentence behind an earlier `else if`.
    let named = SirenSettings {
        master_db: f32::NAN,
        max_voices: 0,
        buses: vec![
            music(),
            music(),                                    // listed more than once
            BusLevel::new("  "),                        // no name
            BusLevel::new("Effects").gain_db(f32::NAN), // a gain that is not a number
        ],
        // **Every rule here carries a hold and a fade the sentence under test is not about**, because `Duck::new`
        // leaves both at `0.0` and would otherwise trip the "no hold and no fade" check from a rule whose real fault is
        // something else — the assertion would pass for the wrong reason. That check has its own row below.
        ducks: vec![
            // starts on a bus the table does not have
            Duck::new("Nope", "Music").hold_s(0.25),
            // lowers a bus the table does not have
            Duck::new("Music", "Nope").hold_s(0.25),
            // raises rather than lowers
            Duck::new("Music", "Effects").by_db(3.0).hold_s(0.25),
            // a hold that is negative
            Duck { hold_s: -1.0, fade_s: 0.4, ..Duck::new("Music", "Effects") },
            // no hold and no fade: the shape `Duck::new` gives a rule before a caller chains one on
            Duck::new("Music", "Music"),
        ],
        ..SirenSettings::default()
    };
    let reported = named.problems();
    for (needle, what) in [
        ("max_voices is 0", "a budget of zero"),
        ("master_db is not a number", "a master that is not a number"),
        ("a bus has no name", "a bus with no name"),
        ("`Music` is listed more than once", "a duplicate bus"),
        ("bus `Effects` has a gain that is not a number", "a bus gain that is not a number"),
        ("a duck starts on bus `Nope`", "a duck starting on a bus that is not there"),
        ("a duck lowers bus `Nope`", "a duck lowering a bus that is not there"),
        ("raises it by 3", "a duck that raises"),
        ("has a hold or fade that is negative or not a number", "a duck hold that is negative"),
        (
            "the duck on `Music` has no hold and no fade, so it lowers the bus for no time at all",
            "a duck with neither a hold nor a fade",
        ),
    ] {
        assert!(reported.iter().any(|s| s.contains(needle)), "the report names {what}: {reported:?}");
    }

    // **Each sentence is its own fault, not a side effect of another.** A table with only the duplicate bus reports
    // only the duplicate, so no assertion above can pass because some unrelated fault happened to print the words.
    let only_duplicate = settings(vec![music(), music()], vec![], 0.0, 8).problems();
    assert_eq!(only_duplicate.len(), 1, "{only_duplicate:?}");
    assert!(only_duplicate[0].contains("listed more than once"), "{only_duplicate:?}");

    // **A duck whose hold is not a number** — the other half of the last sentence, which `-1.0` alone would not cover.
    let nan_hold =
        settings(vec![music()], vec![Duck { fade_s: f32::NAN, ..Duck::new("Music", "Music") }], 0.0, 8);
    assert!(
        nan_hold.problems().iter().any(|s| s.contains("hold or fade that is negative or not a number")),
        "{:?}",
        nan_hold.problems()
    );

    // **And a good table says nothing**, which is what makes every sentence above a fault rather than a habit.
    assert!(SirenSettings::default().problems().is_empty(), "{:?}", SirenSettings::default().problems());
}

/// **Several requests in one frame respect the bus cap together.** A snapshot of the live voices taken before the
/// loop lets three requests in one frame each see an empty bus and all three play, which is the cap broken by the
/// very frame that asked most for it. The budget is re-counted per request, including the voices this frame made.
#[test]
fn three_requests_in_one_frame_fill_a_cap_of_two_with_no_third() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Effects").max_voices(2)],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![cue("hit", "Effects", 0, 0)]);
    let mut app = app(table, settings, sounds(&["hit"]), false);

    for _ in 0..3 {
        app.world_mut().write_message(Play::cue("hit"));
    }
    app.update();

    let answers = played(&mut app);
    assert_eq!(live_voices(&mut app), 2, "two voices sound, whichever frame asked: {answers:?}");
    let stole = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why.starts_with("stolen by")))
        .count();
    assert_eq!(stole, 1, "the third request took a voice back rather than sounding a third: {answers:?}");
}

/// **A full table budget names no bus.** `VoiceLimit::bus` is documented "the bus that had no room, *when one did*";
/// with `max_voices: 1` the whole table is full and every bus has room, so naming the cue's bus sends a person to a
/// bus row they will find empty. The unfixed tree said `Some("Effects")` here.
#[test]
fn a_full_table_budget_names_no_bus() {
    // The bus's own effective limit is the table's (`max_voices: 0` means "the table's"), so the bus cap is tested
    // *first* and the table's cap can never be reached through one bus alone. Two buses, each with room, and a table
    // budget of one: the second play fills the table, and no bus is the subject.
    let settings = SirenSettings {
        max_voices: 1,
        buses: vec![BusLevel::new("Effects"), BusLevel::new("Ambience")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![
        Cue { sounds: vec![SoundSlot::new("line")], priority: 200, max_instances: 0, ..Cue::new("line") },
        Cue {
            sounds: vec![SoundSlot::new("bed")],
            bus: "Ambience".to_owned(),
            priority: 200,
            max_instances: 0,
            ..Cue::new("bed")
        },
    ]);
    let mut app = app(table, settings, sounds(&["line", "bed"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    app.world_mut().write_message(Play::cue("bed"));
    app.update();

    let refusals = refused(&mut app);
    let voice = refusals
        .iter()
        .find(|r| matches!(r.reason, Refusal::VoiceLimit { .. }))
        .expect("the second is refused");
    assert_eq!(voice.reason, Refusal::VoiceLimit { bus: None }, "the table's budget refused, not the bus's");
    assert_eq!(voice.reason.to_string(), "no voice free");
}

/// **A cue at its own polyphony names no bus.** Its `max_instances` refused, and its bus has room.
#[test]
fn a_cue_at_its_own_polyphony_names_no_bus() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    // Unstealable, so the cue's own cap cannot be made room in by taking its oldest voice.
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("line")],
        priority: 200,
        max_instances: 1,
        ..Cue::new("line")
    }]);
    let mut app = app(table, settings, sounds(&["line"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    app.world_mut().write_message(Play::cue("line"));
    app.update();

    let voice = refused(&mut app)
        .into_iter()
        .find(|r| matches!(r.reason, Refusal::VoiceLimit { .. }))
        .expect("the second is refused");
    assert_eq!(voice.reason, Refusal::VoiceLimit { bus: None }, "the cue's polyphony refused, not the bus's");
    assert_eq!(voice.reason.to_string(), "no voice free");
}

/// **A full bus names that bus** — the one case where the bus *is* the subject. Pairs with the two above: a build that
/// stopped naming the bus altogether is red here.
#[test]
fn a_full_bus_names_that_bus() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Effects").max_voices(1)],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("line")],
        priority: 200,
        max_instances: 0,
        ..Cue::new("line")
    }]);
    let mut app = app(table, settings, sounds(&["line"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    app.world_mut().write_message(Play::cue("line"));
    app.update();

    let voice = refused(&mut app)
        .into_iter()
        .find(|r| matches!(r.reason, Refusal::VoiceLimit { .. }))
        .expect("the second is refused");
    assert_eq!(voice.reason, Refusal::VoiceLimit { bus: Some("Effects".to_owned()) });
    assert_eq!(voice.reason.to_string(), "no voice free on bus `Effects`");
}

/// The sentence a panel prints and the payload it matches on cannot drift: each `VoiceLimit` shape has one line.
#[test]
fn every_voice_limit_sentence_matches_its_payload() {
    assert_eq!(Refusal::VoiceLimit { bus: None }.to_string(), "no voice free");
    assert_eq!(
        Refusal::VoiceLimit { bus: Some("Effects".to_owned()) }.to_string(),
        "no voice free on bus `Effects`"
    );
}

/// **A bus that leaves the table leaves the mix.** `SirenMix::db` and `ducked` were written by insertion and never
/// pruned, so a project that swapped `SirenSettings` — a cutscene mix with two buses, a menu with one — kept every bus
/// from the previous table at its last level: `bus_db` answered for a bus that did not exist and `deepest_duck` could
/// name one forever. `SirenMix` is documented as *this frame's* mix.
#[test]
fn a_bus_that_left_the_table_leaves_the_mix() {
    let mut app = app(CueTable::new(vec![]), SirenSettings::default(), sounds(&[]), false);
    app.update();
    // Ambience is ducked while it is still in the table.
    app.world_mut().resource_mut::<bevy_siren::DuckClock>().manual("Ambience", -12.0, 0.0, 5.0, 1.0);
    app.update();
    assert_eq!(app.world().resource::<SirenMix>().deepest_duck().map(|(b, _)| b), Some("Ambience"));

    // The table drops it, and every duck rule with it.
    app.world_mut().insert_resource(SirenSettings {
        buses: vec![BusLevel::new("Music"), BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    });
    for _ in 0..4 {
        app.update();
    }

    let mix = app.world().resource::<SirenMix>().clone();
    let mut names: Vec<&str> = mix.db.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["Effects", "Music"], "this frame's mix has this frame's buses: {names:?}");
    assert!(
        (mix.bus_db("Ambience") - bevy_siren::model::SILENT_DB).abs() < 1e-4,
        "a bus the mix does not have reads as silence, not as its level from another table: {}",
        mix.bus_db("Ambience")
    );
    assert_eq!(
        mix.deepest_duck(),
        None,
        "a duck on a bus that left the table is not this frame's duck: {mix:?}"
    );
}

/// **A bus that stays keeps its smooth approach** — the non-regression a wholesale rebuild would break. The prune
/// leaves the previous value readable, so a bus moving from 0 dB to −6 dB is still *between* them after one frame.
#[test]
fn a_bus_that_stays_keeps_its_smooth_approach() {
    let mut app = app(
        CueTable::new(vec![]),
        SirenSettings { buses: vec![BusLevel::new("Music")], ducks: Vec::new(), ..SirenSettings::default() },
        sounds(&[]),
        false,
    );
    app.update();
    assert!((app.world().resource::<SirenMix>().bus_db("Music") - 0.0).abs() < 1e-4);

    // The clock must tick into the frame that applies the change: a frame with no elapsed time returns the target
    // outright (`approach`'s `dt <= 0` branch), which is a different behaviour and not what this test is about.
    advance(&mut app, 0.016);
    app.world_mut().resource_mut::<SirenSettings>().buses[0].gain_db = -12.0;
    app.update();
    let db = app.world().resource::<SirenMix>().bus_db("Music");
    assert!(db < -0.01 && db > -11.9, "one frame is part of the way, not all of it: {db}");
}

/// **An empty bus list means the crate's four**, for the mix as well as for `answer`. `bus_names()` promises them, and
/// `answer` accepts a cue on `Effects` because of that promise — but `update_mix` iterated `settings.buses` directly, so
/// a table listing none mixed **no buses at all**: the accepted play ran at −80 dB with no refusal.
#[test]
fn an_empty_bus_list_still_means_the_crates_four() {
    let mut app = app(
        CueTable::new(vec![]),
        SirenSettings { buses: vec![], ducks: Vec::new(), ..SirenSettings::default() },
        sounds(&[]),
        false,
    );
    app.update();
    let mut names: Vec<&str> = app.world().resource::<SirenMix>().db.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["Ambience", "Dialogue", "Effects", "Music"], "{names:?}");
}

/// **A voice at `UNSTEALABLE_PRIORITY` is never taken — whatever arrives.** Gregory §14.5.8.4: a cue marked unstealable
/// is one that *"plays, even despite their priority settings"*. The existing table-budget test asks the same
/// unstealable cue for a third voice, so it proves only that a full budget of unstealable voices refuses another
/// unstealable one; this asks a **stealable** request to take one, which is the direction that matters.
#[test]
fn an_unstealable_voice_is_never_taken_by_a_stealable_request() {
    let settings = SirenSettings {
        max_voices: 1,
        buses: vec![BusLevel::new("Music")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![
        cue("line", "Music", 200, 0),
        // Priority 0: the lowest a cue can be, and still not allowed to take an unstealable voice.
        cue("hit", "Music", 0, 0),
    ]);
    let mut app = app(table, settings, sounds(&["line", "hit"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    let line = app
        .world_mut()
        .query::<(&Voice, Entity)>()
        .iter(app.world())
        .find(|(v, _)| v.cue == "line")
        .map(|(_, e)| e)
        .expect("the unstealable voice is live");

    app.world_mut().write_message(Play::cue("hit"));
    app.update();

    assert!(
        refused(&mut app).iter().any(|r| matches!(r.reason, Refusal::VoiceLimit { .. })),
        "a stealable request against a full budget of unstealable voices is refused"
    );
    assert_eq!(live_voices(&mut app), 1, "nothing plays beside it");
    assert!(
        app.world().get::<Voice>(line).is_some(),
        "and the voice a player is listening to is still there"
    );
    assert!(
        !app.world().resource::<SirenLog>().iter().any(|e| matches!(e, Entry::Stopped { cue, why, .. }
            if cue == "line" && why.starts_with("stolen by"))),
        "a steal that could not be made is not logged as one: {:?}",
        app.world().resource::<SirenLog>().iter().map(Entry::line).collect::<Vec<_>>()
    );
}

/// **A stealable voice still plays beside an unstealable one** — the pair that stops the test above from passing
/// vacuously. A build that refused too much would be green there and red here, which is the rule `cues.yaml` states for
/// cases 10–13.
#[test]
fn a_stealable_voice_plays_beside_an_unstealable_one() {
    let settings = SirenSettings {
        max_voices: 2,
        buses: vec![BusLevel::new("Music")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![cue("line", "Music", 200, 0), cue("hit", "Music", 0, 0)]);
    let mut app = app(table, settings, sounds(&["line", "hit"]), false);

    app.world_mut().write_message(Play::cue("line"));
    app.update();
    app.world_mut().write_message(Play::cue("hit"));
    app.update();

    let answers = played(&mut app);
    assert!(answers.iter().any(|p| p.cue == "hit"), "the stealable request plays: {answers:?}");
    assert_eq!(live_voices(&mut app), 2, "two voices, one of each");
    let log = app.world().resource::<SirenLog>();
    assert!(
        !log.iter().any(|e| matches!(e, Entry::Stopped { why, .. } if why.starts_with("stolen by"))),
        "and nothing was taken to make room: {:?}",
        log.iter().map(Entry::line).collect::<Vec<_>>()
    );
}

/// **A cap lowered under live voices does not end them.** `Cue::max_instances` is a **budget on plays**, not a standing
/// count: the cap decides whether the *next* play may start, and a voice it admitted lives out its length. A table is a
/// resource a game may replace at any time, so this is the ordinary path — and the alternative, ending a sound on a
/// frame the game asked for nothing, is the worse surprise.
#[test]
fn a_cap_lowered_under_live_voices_does_not_end_them() {
    let settings = SirenSettings {
        max_voices: 8,
        buses: vec![BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    // Unstealable, so the lowered cap cannot be met by taking one of them — it must refuse instead.
    let table = CueTable::new(vec![Cue { mode: bevy_siren::Mode::Loop, ..cue("hum", "Effects", 200, 2) }]);
    let mut app = app(table, settings, sounds(&["hum"]), false);

    for _ in 0..2 {
        app.world_mut().write_message(Play::cue("hum"));
        app.update();
        played(&mut app);
    }
    assert_eq!(live_voices(&mut app), 2, "the cue's cap of 2 admitted both");

    // The table shrinks the cap under the live voices.
    app.world_mut().resource_mut::<CueTable>().cues[0].max_instances = 1;
    app.world_mut().write_message(Play::cue("hum"));
    app.update();

    assert!(
        refused(&mut app).iter().any(|r| matches!(r.reason, Refusal::VoiceLimit { .. })),
        "the next play is refused by the new cap"
    );
    assert_eq!(live_voices(&mut app), 2, "and the two sounding voices are untouched");
    assert!(
        !app.world().resource::<SirenLog>().iter().any(|e| matches!(e, Entry::Stopped { .. })),
        "nothing was ended to enforce a count nobody asked to enforce: {:?}",
        app.world().resource::<SirenLog>().iter().map(Entry::line).collect::<Vec<_>>()
    );
}

/// **A cap raised under live voices lets the next play through** — the pair that stops the test above from passing
/// vacuously, since a build that refused every play would be green there and red here.
#[test]
fn a_cap_raised_under_live_voices_lets_the_next_play_through() {
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
    assert_eq!(played(&mut app).len(), 1, "the first play is admitted");

    app.world_mut().resource_mut::<CueTable>().cues[0].max_instances = 2;
    app.world_mut().write_message(Play::cue("hum"));
    app.update();

    assert_eq!(played(&mut app).len(), 1, "the raised cap admits the second play");
    assert_eq!(live_voices(&mut app), 2);
}

/// **A table that lists no buses means the crate's four — and the report must know it too.**
///
/// `docs/guide/buses.md`: *"A table that lists no buses means the crate's four (`DEFAULT_BUSES`) **everywhere** — for a
/// play's bus check, for the panel, and for the mix."* `bus_names()` is that one answer. `problems()` built its own set
/// from `self.buses`, so a table the crate **mixes correctly** was reported as four missing buses — the worst thing a
/// report can do, because it teaches an author to ignore it. (Iteration 10 closed the identical bypass in `update_mix`.)
#[test]
fn an_empty_bus_list_reports_no_bus_problems() {
    let table = SirenSettings { buses: vec![], ..SirenSettings::default() };
    assert_eq!(table.bus_names().len(), 4, "the crate's four");
    let problems = table.problems();
    assert!(
        !problems.iter().any(|p| p.contains("bus")),
        "and not one of them is reported missing: {problems:?}"
    );
}

/// **A bus named by two duck rules is reported once.** `default_ducks()` is `Dialogue → Music` and `Dialogue →
/// Effects`, so `Dialogue` is the `source` of both: the ordinary two-bus menu table produced the same sentence twice,
/// and a panel that prints `problems()` verbatim read that as two faults.
#[test]
fn the_default_ducks_name_their_missing_source_once() {
    let table = SirenSettings {
        buses: vec![BusLevel::new("Music"), BusLevel::new("Effects")],
        ..SirenSettings::default()
    };
    let problems = table.problems();
    let dialogue: Vec<&String> = problems.iter().filter(|p| p.contains("`Dialogue`")).collect();
    assert_eq!(dialogue.len(), 1, "one sentence for one missing bus: {problems:?}");
    assert!(dialogue[0].contains("starts on bus"), "and it says which side: {}", dialogue[0]);
}

/// **A duck naming a bus that really is absent still says so, once, with the right words.** The "must not happen"
/// half: a report that stopped reporting would also pass the two tests above.
#[test]
fn a_duck_naming_a_missing_bus_is_still_reported() {
    let table = SirenSettings {
        buses: vec![BusLevel::new("Music")],
        ducks: vec![Duck::new("Ghost", "Music"), Duck::new("Ghost", "Music")],
        ..SirenSettings::default()
    };
    let problems = table.problems();
    let ghost: Vec<&String> = problems.iter().filter(|p| p.contains("`Ghost`")).collect();
    assert_eq!(ghost.len(), 1, "one sentence for two rules naming one absent bus: {problems:?}");
    assert_eq!(
        ghost[0], "a duck starts on bus `Ghost`, which the table does not have",
        "and the sentence is the one it always was"
    );
}

/// **The guard: the crate's default settings report nothing.** The empty-list fix must not turn a clean table into a
/// noisy one — the default table lists all four buses, so there is nothing to say.
#[test]
fn a_default_table_reports_nothing() {
    assert!(SirenSettings::default().problems().is_empty(), "{:?}", SirenSettings::default().problems());
}

/// **`problems_in` agrees with the mix; `problems` assumes the crate's four.**
///
/// A project that lists its own buses in `SirenSettings::buses` is fully supported — `answer`'s `UnknownBus` check,
/// `update_mix` and `SirenSettings::problems()` all read `bus_names()`. `CueTable::problems()` is a convenience wrapper
/// over `problems_with(&DEFAULT_BUSES)`, so it reports every cue on a custom bus as `UnknownBus`; an editor that draws
/// `problems()` (the Cauldron's audio panel does) showed a false fault for every cue in such a project.
#[test]
fn problems_in_reads_the_tables_own_buses() {
    use bevy_siren::BusLevel;
    use bevy_siren::mix::SirenSettings;

    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("a")],
        bus: "Sfx".to_owned(),
        ..Cue::new("shot")
    }]);
    let settings = SirenSettings {
        buses: vec![BusLevel::new("Sfx"), BusLevel::new("Voices")],
        ..SirenSettings::default()
    };

    assert!(
        table.problems().iter().any(|p| matches!(p, bevy_siren::CueProblem::UnknownBus { .. })),
        "the wrapper assumes the crate's four, so it cannot see `Sfx`: {:?}",
        table.problems()
    );
    assert!(table.problems_in(&settings).is_empty(), "{:?}", table.problems_in(&settings));

    // And a table with no buses listed means the crate's four, so `problems_in` must agree with the wrapper there.
    let plain = SirenSettings { buses: vec![], ..SirenSettings::default() };
    let effects = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("a")],
        bus: "Effects".to_owned(),
        ..Cue::new("shot")
    }]);
    assert!(effects.problems_in(&plain).is_empty(), "{:?}", effects.problems_in(&plain));
    assert_eq!(effects.problems_in(&plain), effects.problems(), "both agree on the default four");
}

/// **A duck ends its hold when the voice that justified it is gone.**
///
/// `Duck`'s own doc: *"a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it recover over
/// `fade_s`"* — the duck is a rule about a **voice**. Stopped on frame 0, the measured envelope used to hold `Music` at
/// −8 dB for another 15 frames (the full 0.25 s hold) and then spend 0.6 s recovering: **0.85 s of duck for a sound that
/// had already stopped**. That is the "duck that does not recover" the mix exists to avoid, arriving late instead of
/// never.
#[test]
fn a_duck_ends_its_hold_when_its_source_is_stopped() {
    let mut app = app(
        CueTable::new(vec![cue("line", "Dialogue", 200, 1)]),
        SirenSettings::default(),
        sounds(&["line"]),
        false,
    );
    app.world_mut().write_message(Play::cue("line"));
    app.update();
    app.update();

    // The line is stopped: its voice is gone from what a player hears.
    app.world_mut().write_message(StopCue { cue: "line".into() });
    run_for(&mut app, 0.10, 1.0 / 60.0);

    let ducked = app.world().resource::<SirenMix>().ducked.get("Music").copied().unwrap_or(0.0);
    assert!(
        ducked > -7.5,
        "ten frames after the stop the duck is already recovering, not still holding: {ducked}"
    );
    assert!(ducked < -0.05, "and it has not jumped to zero in one frame: {ducked}");
}

/// **A duck whose source keeps playing holds for its whole `hold_s`.** The guard: the fix must not cut a hold that is
/// still justified by a voice a player can hear. `Mode::Loop` keeps the voice alive, so the hold is the authored 0.25 s.
#[test]
fn a_duck_whose_source_keeps_playing_holds() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("line")],
        bus: "Dialogue".to_owned(),
        priority: 200,
        mode: Mode::Loop,
        ..Cue::new("line")
    }]);
    let mut app = app(table, SirenSettings::default(), sounds(&["line"]), false);
    app.world_mut().write_message(Play::cue("line"));
    run_for(&mut app, 0.20, 1.0 / 60.0);

    let ducked = app.world().resource::<SirenMix>().ducked.get("Music").copied().unwrap_or(0.0);
    assert!(
        (ducked - -8.0).abs() < 1e-5,
        "at 0.20 s the hold (0.25 s) has not elapsed, so the duck is still full: {ducked}"
    );
}

/// **A second line mid-hold extends it.** `DuckClock::fire_all` re-arms on each play, and the rewind must not undo that:
/// two voices on the source bus mean the hold restarts.
#[test]
fn a_second_line_extends_the_duck() {
    let mut app = app(
        CueTable::new(vec![Cue {
            sounds: vec![SoundSlot::new("line")],
            bus: "Dialogue".to_owned(),
            priority: 200,
            max_instances: 3,
            mode: Mode::Loop,
            ..Cue::new("line")
        }]),
        SirenSettings::default(),
        sounds(&["line"]),
        false,
    );
    app.world_mut().write_message(Play::cue("line"));
    run_for(&mut app, 0.20, 1.0 / 60.0);
    app.world_mut().write_message(Play::cue("line"));
    run_for(&mut app, 0.20, 1.0 / 60.0);

    let ducked = app.world().resource::<SirenMix>().ducked.get("Music").copied().unwrap_or(0.0);
    assert!(
        (ducked - -8.0).abs() < 1e-5,
        "the second line restarted the hold, so 0.20 s later it is still full: {ducked}"
    );
}

/// **The newest `Ducked` entry naming a bus is the one a panel should show**, and a later duck for another bus must not
/// be mistaken for it.
///
/// `examples/mixer`'s bus row reads `duck -8.0 dB by dialogue.greeting` — it used to read only `duck -8.0 dB`, so a person
/// had to find the matching log line to learn *which* cue lowered the bus, while `Entry::Ducked` exists to say exactly
/// that. The panel finds it with `SirenLog::recent(..)` and a match on `bus`; this pins the rule, because the example sits
/// outside the suite and a change to `Entry` or `recent` could break the panel silently.
#[test]
fn the_newest_duck_naming_a_bus_is_the_one_a_panel_shows() {
    let mut log = SirenLog::new();
    log.push(Entry::Ducked {
        at_s: 0.1,
        bus: "Music".to_owned(),
        by_db: -8.0,
        by: "dialogue.greeting".to_owned(),
    });
    log.push(Entry::Played {
        at_s: 0.2,
        cue: "footstep.grass".to_owned(),
        sound: "step_01".to_owned(),
        bus: "Effects".to_owned(),
        gain_db: 0.0,
    });
    log.push(Entry::Ducked {
        at_s: 0.3,
        bus: "Effects".to_owned(),
        by_db: -4.0,
        by: "other.line".to_owned(),
    });

    fn by(log: &SirenLog, bus: &str) -> Option<String> {
        log.recent(SirenLog::CAPACITY).find_map(|entry| match entry {
            Entry::Ducked { bus: b, by, .. } if b == bus => Some(by.clone()),
            _ => None,
        })
    }
    assert_eq!(
        by(&log, "Music").as_deref(),
        Some("dialogue.greeting"),
        "a non-duck entry between them does not confuse it"
    );
    assert_eq!(by(&log, "Effects").as_deref(), Some("other.line"));
    assert_eq!(by(&log, "Ambience"), None, "a bus nothing ducked has no name to show");

    // A second duck on the same bus wins: the row must say what is lowering it *now*.
    log.push(Entry::Ducked { at_s: 0.4, bus: "Music".to_owned(), by_db: -9.5, by: "stinger".to_owned() });
    assert_eq!(by(&log, "Music").as_deref(), Some("stinger"), "the newest one wins");
}

/// **A duck built by `Duck::new` alone ducks for no time at all, and nothing reports it.**
///
/// `Duck::new("A", "B")` leaves `hold_s: 0.0` and `fade_s: 0.0`, and `active_duck_db` returns `0.0` once
/// `elapsed >= hold` when `fade <= 0.0` — so the rule fires, is recorded, and is inaudible on the frame it fires.
/// The crate's own `default_ducks()` chains `.hold_s(0.25).fade_s(…)` on every rule, which is how the pair every
/// project starts from works; a hand-written rule that forgets both is silent, and `problems()` — nine sentences —
/// says nothing.
///
/// This test **pins the measured behaviour** so the hazard is recorded rather than assumed. It is not an endorsement:
/// a sentence for it is `docs/loop/needs-decision/zero-length-duck.md`.
#[test]
fn a_duck_with_no_hold_and_no_fade_ducks_for_no_time() {
    let settings = SirenSettings {
        buses: vec![BusLevel::new("Music"), BusLevel::new("Dialogue")],
        ducks: vec![Duck::new("Dialogue", "Music").by_db(-8.0)],
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue { bus: "Dialogue".to_owned(), ..cue("line", "Dialogue", 0, 1) }]);
    let mut silent = app(table, settings, sounds(&["line"]), false);

    silent.world_mut().write_message(Play::cue("line"));
    silent.update();
    assert_eq!(
        silent.world().resource::<SirenMix>().ducked.get("Music").copied(),
        Some(0.0),
        "the rule fired on the frame the line started, and its depth is already spent"
    );

    // The same rule with the crate's own hold and fade does duck, which is what makes this a hazard rather than a law.
    let settings = SirenSettings {
        buses: vec![BusLevel::new("Music"), BusLevel::new("Dialogue")],
        ducks: vec![Duck::new("Dialogue", "Music").by_db(-8.0).hold_s(0.25).fade_s(0.6)],
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue { bus: "Dialogue".to_owned(), ..cue("line", "Dialogue", 0, 1) }]);
    let mut held = app(table, settings, sounds(&["line"]), false);
    held.world_mut().write_message(Play::cue("line"));
    held.update();
    assert_eq!(
        held.world().resource::<SirenMix>().ducked.get("Music").copied(),
        Some(-8.0),
        "with a hold it reaches its authored depth"
    );
}
