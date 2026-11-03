//! **Case 31**: a number the crate had to rewrite is said out loud.
//!
//! The behaviour was already right — a non-finite gain is read as no gain, the same rule `distance_gain` follows for a
//! distance — and it was silent. A host inspecting `problems()` or `Refused` could not tell that an author had typed
//! something the crate neutralised, so the cue sounded at its authored level and nobody ever learned why the number
//! they wrote had no effect.
//!
//! The report goes through `Entry::Stopped`'s free-text `why`, so **no public enum gains a variant** and no consumer's
//! exhaustive `match` breaks. That is `needs-decision/bad-gain-problem.md`'s recommendation C, and these are its tests.

use bevy::prelude::*;
use bevy_siren::*;

/// A headless app with the crate's plugin, the sound loaded, and one cue whose gain is whatever the test says.
fn app_with(gain_db: f32) -> App {
    let mut table = CueTable::default();
    table.insert(Cue { sounds: vec![SoundSlot::new("step_01")], gain_db, ..Cue::new("footstep.grass") });
    crate::helpers::app(table, SirenSettings::default(), crate::helpers::sounds(&["step_01"]), false)
}

/// The neutralisation sentences the log carries, which is what a panel shows.
fn neutralised(app: &App) -> Vec<String> {
    app.world()
        .resource::<SirenLog>()
        .iter()
        .filter_map(|e| match e {
            Entry::Stopped { why, .. } if why.contains("not a number") => Some(why.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_cue_gain_that_is_not_a_number_is_reported_beside_its_play() {
    // **Case 31.** The cue plays — neutralising is the behaviour, not a refusal — and the log says which field was
    // rewritten and what it was read as. Before this, the answer was silence.
    let mut app = app_with(f32::NAN);
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let said = neutralised(&app);
    assert_eq!(said.len(), 1, "one sentence for one rewritten number, got {said:?}");
    let sentence = &said[0];
    assert!(sentence.contains("cue gain_db"), "it names the field: {sentence}");
    assert!(sentence.contains("was not a number"), "and says what was wrong: {sentence}");
    assert!(sentence.contains("neutral value"), "and what it did: {sentence}");

    // **The play still happens.** The cue is audible at the level it actually authored, which is the promise
    // `bad-gain-problem.md` makes: a lost sound is worse than a mis-typed number.
    let played: Vec<&Entry> =
        app.world().resource::<SirenLog>().iter().filter(|e| matches!(e, Entry::Played { .. })).collect();
    assert_eq!(played.len(), 1, "the cue plays, and is not refused");
    match played[0] {
        Entry::Played { gain_db, .. } => {
            assert!(gain_db.is_finite(), "and reports a number the device can play, got {gain_db}");
            assert_eq!(*gain_db, 0.0f32, "a NaN reads as no gain");
        }
        other => panic!("expected Played, got {other:?}"),
    }
}

#[test]
fn a_finite_gain_says_nothing_at_all() {
    // **The report that must NOT fire.** A cue with an ordinary gain logs its play and nothing else; a sentence on
    // every play would be noise a person learns to skip.
    let mut app = app_with(-6.0);
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();
    assert_eq!(neutralised(&app), Vec::<String>::new(), "a finite gain is not a mistake");
}

#[test]
fn the_sentence_is_grammatical_for_one_field_and_for_several() {
    // Both a sound's gain and the play's own can be wrong at once, and the sentence reads as English either way.
    let mut table = CueTable::default();
    table.insert(Cue {
        sounds: vec![SoundSlot { sound: "step_01".into(), gain_db: f32::INFINITY }],
        ..Cue::new("footstep.grass")
    });
    let mut app =
        crate::helpers::app(table, SirenSettings::default(), crate::helpers::sounds(&["step_01"]), false);
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let said = neutralised(&app);
    assert_eq!(said.len(), 1);
    assert!(said[0].contains("sound gain_db"), "the sound's own gain is named: {}", said[0]);
    assert!(said[0].contains("was not a number"), "one field takes the singular: {}", said[0]);
}

#[test]
fn the_play_still_reports_the_gain_the_device_will_use() {
    // The meter and the device must agree — the rule iteration 62 established for pitch. A `Played` carrying a NaN
    // while the device played at 0 dB is the disagreement this crate refuses.
    let mut table = CueTable::default();
    table.insert(Cue {
        sounds: vec![SoundSlot { sound: "step_01".into(), gain_db: f32::NAN }],
        gain_db: -3.0,
        ..Cue::new("footstep.grass")
    });
    let mut app =
        crate::helpers::app(table, SirenSettings::default(), crate::helpers::sounds(&["step_01"]), false);
    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let played = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .find_map(|e| match e {
            Entry::Played { gain_db, .. } => Some(*gain_db),
            _ => None,
        })
        .expect("the cue played");
    assert!(played.is_finite(), "the reported gain is a number: {played}");
    assert!(
        (played + 3.0).abs() < 1e-6,
        "and it is the cue's own -3 dB, with the NaN sound read as 0, got {played}"
    );
}
