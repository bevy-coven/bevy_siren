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

/// **A stinger's duck holds for the stinger, then releases.**
///
/// `Stinger::duck_s` is documented as *"how long the duck holds before it starts recovering"*, and the music must step
/// back for as long as the punctuation mark sounds. The unfixed tree passed `duck_s` as the **fade** with a hold of
/// zero, so `Music` began climbing on the frame the stinger fired: at the halfway point of a 0.6 s stinger it was back
/// at half depth. The crate's own `active_duck_db` shape — full for the hold, then a linear recovery — is what this
/// pins, and `DEFAULT_FADE_S` is the release after the hold.
#[test]
fn a_stinger_holds_the_music_back_for_its_whole_length() {
    use bevy_siren::model::DEFAULT_FADE_S;

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
    advance(&mut app, 1.0 / 60.0);
    app.world_mut().write_message(Stinger::new("hit").duck_db(-9.0).duck_s(0.6));
    app.update();
    let fired = app.world().resource::<SirenMix>().ducked["Music"];
    assert!((fired - -9.0).abs() < 1e-4, "the duck is at full depth on the frame it fires: {fired}");

    // Inside the hold: still the full depth. This is the assertion the unfixed tree fails.
    run_for(&mut app, 0.5, 1.0 / 60.0);
    let held = app.world().resource::<SirenMix>().ducked["Music"];
    assert!((held - -9.0).abs() < 0.15, "halfway through a 0.6 s stinger the music is still held: {held}");

    // After the hold plus the release, the duck is over.
    run_for(&mut app, 0.4, 1.0 / 60.0);
    let released = app.world().resource::<SirenMix>().ducked["Music"];
    assert!(released > -0.01, "the music is back to 0 dB by hold + release ({DEFAULT_FADE_S} s): {released}");
}

/// **A layer takes the fade it was given.** `MusicLayers::fade_s` is documented as *"how long the move takes once it
/// starts"*, and the unfixed tree eased every move at `MIX_TAU_S` (~150 ms), so an authored 0.8 s crossfade arrived in
/// a blink with nothing said about it. This pins the authored duration at both ends: still moving at 0.3 s, arrived by
/// `fade_s`.
#[test]
fn a_layer_takes_the_fade_it_was_given() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    app.update();
    assert_eq!(app.world().resource::<MusicState>().live(), 2, "one voice per layer");
    assert_eq!(app.world().resource::<MusicState>().layer_db("combat"), Some(bevy_siren::model::SILENT_DB));

    // The clock must tick into the frame that applies the change: a frame with no elapsed time holds the gain rather
    // than skipping the fade, which is the crate's rule and the reason this loop advances first.
    advance(&mut app, 1.0 / 60.0);
    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), -9.0)]).now().fade_s(0.8));
    app.update();

    // 0.3 s in: today's code has long settled; a 0.8 s fade is still arriving.
    run_for(&mut app, 0.3, 1.0 / 60.0);
    let early = app.world().resource::<MusicState>().layer_db("combat").expect("the layer");
    assert!(
        early < -12.0,
        "0.3 s into an 0.8 s fade the layer is still well short of -9 dB, not settled: {early}"
    );

    // By the authored duration plus a frame or two, it is there.
    run_for(&mut app, 0.55, 1.0 / 60.0);
    let arrived = app.world().resource::<MusicState>().layer_db("combat").expect("the layer");
    assert!((arrived - -9.0).abs() < 0.05, "0.85 s into an 0.8 s fade the layer has arrived: {arrived}");
}

/// **`fade_s(0)` keeps the crate's `MIX_TAU_S` easing** — the builder's documented default is unchanged by the fix.
#[test]
fn a_layer_fade_of_zero_keeps_the_crates_easing() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    advance(&mut app, 1.0 / 60.0);
    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), -9.0)]).now());
    app.update();
    // `MIX_TAU_S` is 0.05 s, so 0.3 s leaves ~0.17 dB; half a second is well inside the crate's own 0.2 dB tolerance.
    run_for(&mut app, 0.5, 1.0 / 60.0);
    let db = app.world().resource::<MusicState>().layer_db("combat").expect("the layer");
    assert!((db - -9.0).abs() < 0.05, "no fade given settles in ~150 ms, as it always did: {db}");
}

/// **A layer the change does not name keeps the fade it had**, exactly as it keeps its gain.
#[test]
fn a_layer_missing_from_a_change_keeps_its_fade() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();

    // `calm` gets a long fade; `combat` is named in a second change with none, and must still arrive quickly.
    advance(&mut app, 1.0 / 60.0);
    app.world_mut().write_message(MusicLayers::new(vec![("calm".to_owned(), -12.0)]).now().fade_s(0.8));
    app.update();
    advance(&mut app, 1.0 / 60.0);
    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), -9.0)]).now());
    app.update();

    // 0.2 s in: the long fade is nowhere near done; the layer that was given no fade is all but settled.
    run_for(&mut app, 0.2, 1.0 / 60.0);
    let state = app.world().resource::<MusicState>();
    let calm = state.layer_db("calm").expect("calm");
    let combat = state.layer_db("combat").expect("combat");
    assert!(
        calm < combat - 6.0,
        "the layer given a long fade ({calm}) is far behind the one given none ({combat})"
    );
    // `MIX_TAU_S` over 0.2 s is ~-10.3 dB, i.e. 1.3 dB short; the point is that it is far ahead of the long fade.
    assert!(combat > -10.5, "the layer given no fade is nearly there already: {combat}");
}
