//! Music: vertical layering that moves on the bar.
//!
//! A layer is a voice like any other — the plugin writes a [`Play`] and adopts the entity [`Played`] reports — so
//! these tests assert on `Voice`, `MusicState` and `SirenMix`, which is what the audio panel draws.

use bevy::prelude::*;
use bevy_siren::{
    Cue, CueTable, Mode, MusicLayer, MusicLayers, MusicScore, MusicState, Play, SirenMix, SirenSettings,
    SoundSlot, Stinger, Voice, When,
};

use crate::helpers::{advance, app, run_for, sounds, voices};

/// A headless app with a score, the crate's default mix, and the voices the score's layers need.
fn app_with(score: MusicScore, table: CueTable, keys: &[&str]) -> App {
    let mut app = app(table, SirenSettings::default(), sounds(keys), true);
    app.insert_resource(score);
    app
}

fn layer_cue(name: &str) -> Cue {
    Cue {
        sounds: vec![SoundSlot::new(name)],
        bus: "Music".to_owned(),
        mode: Mode::Loop,
        priority: 200,
        ..Cue::new(name)
    }
}

/// A two-layer score at 120 bpm, every layer silent: the shape a game ships.
fn two_layer_score() -> MusicScore {
    MusicScore {
        layers: vec![MusicLayer::new("calm", "track.calm"), MusicLayer::new("combat", "track.combat")],
        tempo_bpm: 120.0,
        beats_per_bar: 4.0,
    }
}

fn two_layer_table() -> CueTable {
    CueTable::new(vec![layer_cue("track.calm"), layer_cue("track.combat")])
}

/// **`MusicLayers { when: NextBar }` at 120 bpm from t = 0.3 s starts no gain change before t = 2.0 s.**
#[test]
fn a_change_waits_for_the_bar() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    assert_eq!(app.world().resource::<MusicState>().live(), 2, "one voice per layer");
    assert_eq!(voices(&mut app), 2);

    // 0.3 s into the score.
    advance(&mut app, 0.3);
    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), 0.0)]).next_bar());
    app.update();
    let state = app.world().resource::<MusicState>();
    let due = state.next_change_s().expect("a change is queued");
    assert!((due - 2.0).abs() < 1e-6, "the next bar at 120 bpm from 0.3 s is 2.0 s: {due}");
    assert_eq!(state.target_db("combat"), Some(bevy_siren::model::SILENT_DB), "nothing moved before the bar");

    // At the bar, the target moves, and the gain eases toward it.
    run_for(&mut app, 1.71, 0.016);
    run_for(&mut app, 0.96, 0.016);
    let state = app.world().resource::<MusicState>();
    assert_eq!(state.target_db("combat"), Some(0.0), "the change landed on the bar");
    let level = state.layer_db("combat").expect("the combat layer");
    assert!(level > -1.0, "the layer eased up to its target: {level}");
}

/// A layer at `SILENT_DB` stays in the mix: it costs one silent voice and keeps every layer phase-aligned.
#[test]
fn a_silent_layer_stays_in_the_mix() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    run_for(&mut app, 0.128, 0.016);
    let state = app.world().resource::<MusicState>();
    assert_eq!(state.live(), 2, "both layers are still playing");
    assert_eq!(state.layer_db("combat"), Some(bevy_siren::model::SILENT_DB));
    assert_eq!(
        app.world_mut().query::<&Voice>().iter(app.world()).filter(|v| v.cue.starts_with("track.")).count(),
        2
    );
}

/// **A stinger ducks the music** by its own amount, and the log says a stinger did it.
#[test]
fn a_stinger_ducks_the_music() {
    let table = CueTable::new(vec![
        layer_cue("track.calm"),
        Cue {
            sounds: vec![SoundSlot::new("hit")],
            bus: "Music".to_owned(),
            priority: 210,
            ..Cue::new("hit")
        },
    ]);
    let mut app = app_with(
        MusicScore {
            layers: vec![MusicLayer::new("calm", "track.calm")],
            tempo_bpm: 120.0,
            beats_per_bar: 4.0,
        },
        table,
        &["track.calm", "hit"],
    );
    app.update();
    app.world_mut().write_message(Stinger::new("hit").duck_db(-9.0).duck_s(1.5));
    app.update();
    let mix = app.world().resource::<SirenMix>().clone();
    assert!((mix.ducked["Music"] - -9.0).abs() < 1e-5, "{:?}", mix.ducked);
    assert!(
        app.world()
            .resource::<bevy_siren::SirenLog>()
            .iter()
            .any(|e| matches!(e, bevy_siren::Entry::Ducked { by, .. } if by == "stinger"))
    );
}

/// A score whose layer is not a loop is a problem, not a silent silence.
#[test]
fn a_score_reports_a_layer_that_cannot_hold() {
    let mut table = two_layer_table();
    table.insert(Cue { mode: Mode::Once, ..layer_cue("track.calm") });
    let problems = two_layer_score().problems(&table);
    assert!(problems.iter().any(|p| p.contains("not a loop")), "{problems:?}");

    let missing = MusicScore {
        layers: vec![MusicLayer::new("calm", "track.absent")],
        tempo_bpm: 120.0,
        beats_per_bar: 4.0,
    };
    assert!(missing.problems(&table).iter().any(|p| p.contains("which the table does not have")));
    assert!(MusicScore::default().problems(&table).iter().any(|p| p.contains("no layers")));
}

/// With no beat grid every transition is immediate: a pad, not a song.
#[test]
fn no_beat_grid_means_no_wait() {
    let mut score = two_layer_score();
    score.tempo_bpm = 0.0;
    assert!(score.bar_s().is_none());
    let mut app = app_with(score, two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.world_mut().write_message(MusicLayers::new(vec![("calm".to_owned(), -3.0)]).next_bar());
    app.update();
    let state = app.world().resource::<MusicState>();
    assert_eq!(state.target_db("calm"), Some(-3.0), "no grid, no wait");
    assert!(state.next_change_s().is_none() || state.next_change_s().is_some_and(|t| t <= 1e-9));
    let _ = Play::cue("track.calm");
}

/// `When::Now` moves on the frame it is asked for, and `When::NextBeat` on the next beat.
#[test]
fn now_moves_at_once_and_next_beat_at_the_beat() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    advance(&mut app, 0.3);
    app.world_mut().write_message(MusicLayers::new(vec![("calm".to_owned(), -6.0)]).now());
    app.update();
    assert_eq!(app.world().resource::<MusicState>().target_db("calm"), Some(-6.0));

    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), -6.0)]).next_beat());
    app.update();
    // The clock has advanced twice by 0.3 s, so this frame is at 0.6 s and the next beat is 1.0 s.
    let due = app.world().resource::<MusicState>().next_change_s().expect("a change is queued");
    assert!((due - 1.0).abs() < 1e-6, "the next beat at 120 bpm from 0.6 s is 1.0 s: {due}");
    let _ = When::NextBeat;
}
