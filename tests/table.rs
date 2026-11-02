//! The cue table as data: what parses, and everything wrong with a table that a person should be told.
#![cfg(feature = "serde")]

use bevy_siren::{Cue, CueProblem, CueTable, Mode, PitchRange, Repeat, SoundSlot};

/// A table with every field omitted except `name` and `sounds` parses, and every default is the documented one.
#[test]
fn a_minimal_cue_parses_with_every_default() {
    let text = r#"(
        cues: [
            (name: "footstep.grass", sounds: [(sound: "step_01"), (sound: "step_02")]),
            (name: "wind", sounds: [(sound: "wind_loop")], mode: Loop, bus: "Ambience"),
        ],
    )"#;
    let table = CueTable::from_ron(text).expect("the table parses");
    let step = table.get("footstep.grass").expect("the first cue");
    assert_eq!(step.bus_name(), "Effects");
    assert_eq!(step.gain_db, 0.0);
    assert_eq!(step.pitch, PitchRange::default());
    assert_eq!(step.repeat, Repeat::AvoidRepeat { avoid: 1 });
    assert_eq!(step.mode, Mode::Once);
    assert_eq!(step.max_instances, 1);
    assert_eq!(step.priority, 0);
    assert!(!step.spatial);
    assert_eq!(step.attenuation, bevy_siren::Attenuation::default());

    let wind = table.get("wind").expect("the second cue");
    assert_eq!(wind.mode, Mode::Loop);
    assert_eq!(wind.bus_name(), "Ambience");
    assert_eq!(wind.sounds[0].gain_db, 0.0);

    // Written back and read again, it is the same table.
    let written = table.to_ron().expect("the table writes");
    assert_eq!(CueTable::from_ron(&written).as_ref(), Ok(&table));
}

/// A file with a field the crate does not know is refused, not silently half-read.
#[test]
fn an_unknown_field_is_refused() {
    let text = r#"(cues: [(name: "a", sounds: [(sound: "s")], reverb: 3.0)])"#;
    let error = CueTable::from_ron(text).expect_err("an unknown field is an error");
    assert!(error.contains("reverb"), "{error}");
}

/// Every problem a table can have is named, in a sentence a person reads on a status line.
#[test]
fn problems_names_each_mistake() {
    let table = CueTable::new(vec![
        Cue::new("silent"),
        Cue { sounds: vec![SoundSlot::new("a")], ..Cue::new("twice") },
        Cue { sounds: vec![SoundSlot::new("b")], ..Cue::new("twice") },
        Cue { sounds: vec![SoundSlot::new("c")], bus: "Reverb".to_owned(), ..Cue::new("wrong_bus") },
        Cue {
            sounds: vec![SoundSlot::new("d")],
            pitch: PitchRange { low: 2.0, high: -2.0 },
            ..Cue::new("inverted")
        },
        Cue { sounds: vec![SoundSlot::new("e")], max_instances: 0, ..Cue::new("unbounded") },
        Cue {
            sounds: vec![SoundSlot::new("f")],
            attenuation: bevy_siren::Attenuation::Linear { max_m: -1.0 },
            ..Cue::new("bad_falloff")
        },
    ]);
    let problems = table.problems();
    let named = |p: &CueProblem, name: &str| match p {
        CueProblem::NoSounds { name: n }
        | CueProblem::DuplicateName { name: n }
        | CueProblem::InvertedPitch { name: n }
        | CueProblem::BadAttenuation { name: n }
        | CueProblem::UnlimitedInstances { name: n } => n == name,
        CueProblem::UnknownBus { name: n, .. } => n == name,
    };
    for (problem, name) in [
        (CueProblem::NoSounds { name: "silent".into() }, "silent"),
        (CueProblem::DuplicateName { name: "twice".into() }, "twice"),
        (CueProblem::UnknownBus { name: "wrong_bus".into(), bus: "Reverb".into() }, "wrong_bus"),
        (CueProblem::InvertedPitch { name: "inverted".into() }, "inverted"),
        (CueProblem::UnlimitedInstances { name: "unbounded".into() }, "unbounded"),
        (CueProblem::BadAttenuation { name: "bad_falloff".into() }, "bad_falloff"),
    ] {
        assert!(problems.contains(&problem), "{problem:?} is not in {problems:#?}");
        assert!(problems.iter().any(|p| named(p, name)), "{name} is not reported");
    }
    // Each problem is a sentence, not a debug print.
    for problem in &problems {
        let line = problem.to_string();
        assert!(line.len() > 20, "`{line}` is not a sentence");
        assert!(!line.contains("CueProblem"), "`{line}` is a debug print");
    }
    // A clean table has nothing to say.
    assert!(
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("a")], ..Cue::new("fine") }])
            .problems()
            .is_empty()
    );
}

/// A table's own bus list is what `problems_with` checks against, so a project with its own buses is not told its
/// cues are wrong.
#[test]
fn problems_with_uses_the_projects_own_buses() {
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("a")],
        bus: "Reverb".to_owned(),
        ..Cue::new("wash")
    }]);
    assert!(!table.problems().is_empty());
    assert!(table.problems_with(&["Effects", "Reverb"]).is_empty());
}

/// `insert` replaces by name and hands back the old cue; `remove` hands back the cue it took; `get` is the lookup.
#[test]
fn the_table_replaces_by_name() {
    let mut table = CueTable::default();
    assert!(table.insert(Cue { sounds: vec![SoundSlot::new("a")], ..Cue::new("step") }).is_none());
    let old = table.insert(Cue { gain_db: -3.0, sounds: vec![SoundSlot::new("b")], ..Cue::new("step") });
    assert_eq!(old.expect("the first cue").sounds[0].sound, "a");
    assert_eq!(table.cues.len(), 1);
    assert_eq!(table.get("step").expect("the replacement").sounds[0].sound, "b");
    assert_eq!(table.names().collect::<Vec<_>>(), vec!["step"]);
    assert_eq!(table.remove("step").expect("removed").sounds[0].sound, "b");
    assert!(table.get("step").is_none());
}

/// The distinct `sound` keys are what a project has to deliver, in table order, each once.
#[test]
fn sounds_lists_each_key_once() {
    let table = CueTable::new(vec![
        Cue { sounds: vec![SoundSlot::new("a"), SoundSlot::new("b")], ..Cue::new("one") },
        Cue { sounds: vec![SoundSlot::new("b"), SoundSlot::new("c")], ..Cue::new("two") },
    ]);
    assert_eq!(table.sounds(), vec!["a", "b", "c"]);
}
