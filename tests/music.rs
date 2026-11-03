//! Music: vertical layering that moves on the bar.
//!
//! A layer is a voice like any other — the plugin writes a [`Play`] and adopts the entity [`Played`] reports — so
//! these tests assert on `Voice`, `MusicState` and `SirenMix`, which is what the audio panel draws.

use bevy::prelude::*;
use bevy_siren::{
    Cue, CueTable, Mode, MusicLayer, MusicLayers, MusicScore, MusicState, Play, SirenLog, SirenMix,
    SirenSettings, SoundSlot, Stinger, StopCue, Voice, When,
};

use crate::helpers::{advance, app, played, refused, run_for, sounds, voices};

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

/// **A stinger whose cue is not in the table ducks nothing.** `play_stingers` wrote the play and armed the duck in the
/// same breath, but `Play` is answered *later in the frame* — so a refused stinger left the `Music` bus at −9 dB for a
/// sound nobody heard, and the log said both "stinger ducked Music by -9.0 dB" and "no cue of that name".
#[test]
fn a_stinger_whose_cue_is_unknown_ducks_nothing() {
    use bevy_siren::Refusal;

    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();

    app.world_mut().write_message(Stinger::new("music.does-not-exist").duck_db(-9.0).duck_s(1.0));
    app.update();

    let mix = app.world().resource::<SirenMix>().clone();
    assert!(
        mix.ducked["Music"] > -0.01,
        "the music stepped back for a stinger that never played: {:?}",
        mix.ducked
    );
    assert!(
        refused(&mut app).iter().any(|r| matches!(r.reason, Refusal::UnknownCue)),
        "the request is refused, and that refusal is the whole story"
    );
    let log: Vec<String> = app.world().resource::<bevy_siren::SirenLog>().iter().map(|e| e.line()).collect();
    assert!(
        !log.iter().any(|line| line.contains("stinger ducked")),
        "no `Ducked` line may contradict the refusal: {log:?}"
    );
}

/// **A stinger whose sound has no handle ducks nothing** — the same bug, reached by a cue that *is* on `Music`.
#[test]
fn a_stinger_whose_sound_is_missing_ducks_nothing() {
    // `music.stinger` is on the Music bus and its key `stinger_01` has no handle in `SirenSounds`.
    let table = CueTable::new(vec![
        layer_cue("track.calm"),
        Cue {
            sounds: vec![SoundSlot::new("stinger_01")],
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
        &["track.calm"],
    );
    app.update();
    app.update();

    app.world_mut().write_message(Stinger::new("hit").duck_db(-9.0).duck_s(1.0));
    app.update();

    let mix = app.world().resource::<SirenMix>().clone();
    assert!(mix.ducked["Music"] > -0.01, "no sound, no duck: {:?}", mix.ducked);
    let log: Vec<String> = app.world().resource::<bevy_siren::SirenLog>().iter().map(|e| e.line()).collect();
    assert!(
        !log.iter().any(|line| line.contains("stinger ducked")),
        "the log states the refusal, never a duck beside it: {log:?}"
    );
    assert!(log.iter().any(|l| l.contains("no handle")), "and the refusal is there: {log:?}");
}

/// **A stinger that plays ducks on the frame it fires** — the case the rewrite must not delay or drop. Same depth, same
/// frame, one `Ducked` whose `by` is `"stinger"`.
#[test]
fn a_stinger_that_plays_ducks_on_the_frame_it_fires() {
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
    app.update();

    app.world_mut().write_message(Stinger::new("hit").duck_db(-9.0).duck_s(0.6));
    app.update();

    let mix = app.world().resource::<SirenMix>().clone();
    assert!(
        (mix.ducked["Music"] - -9.0).abs() < 1e-4,
        "the dip is in the clock before this frame's mix: {:?}",
        mix.ducked
    );
    let ducks = app
        .world()
        .resource::<bevy_siren::SirenLog>()
        .iter()
        .filter(|e| matches!(e, bevy_siren::Entry::Ducked { by, .. } if by == "stinger"))
        .count();
    assert_eq!(ducks, 1, "one stinger, one duck line");
}

/// **A stinger on another bus is still refused, and still ducks nothing** — the existing rule, unchanged by the rewrite.
#[test]
fn a_stinger_on_another_bus_is_still_refused() {
    let table = CueTable::new(vec![
        layer_cue("track.calm"),
        Cue {
            sounds: vec![SoundSlot::new("hit")],
            bus: "Effects".to_owned(),
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
    app.update();

    app.world_mut().write_message(Stinger::new("hit").duck_db(-9.0));
    app.update();

    let mix = app.world().resource::<SirenMix>().clone();
    assert!(
        mix.ducked["Music"] > -0.01,
        "a stinger that cannot be on Music touches nothing: {:?}",
        mix.ducked
    );
    let log: Vec<String> = app.world().resource::<bevy_siren::SirenLog>().iter().map(|e| e.line()).collect();
    assert!(log.iter().any(|l| l.contains("names bus `Effects`")), "{log:?}");
}

/// **A layer removed from the score stops its voice.** `MusicScore` is a resource a project and the Cauldron edit;
/// `start_layers` used to only ever *add* layers, so a removed layer kept a looping voice playing forever — ownerless,
/// because `retire_voices` skips loops by design. The score and the sound disagreed, and nothing said so.
#[test]
fn a_layer_removed_from_the_score_stops_its_voice() {
    use bevy_siren::Entry;

    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<MusicState>().live(), 2, "both stems are running");

    // The game drops one layer from the score.
    app.world_mut().resource_mut::<MusicScore>().layers.retain(|l| l.name != "combat");
    run_for(&mut app, 0.6, 1.0 / 60.0);

    let state = app.world().resource::<MusicState>();
    assert_eq!(state.names().collect::<Vec<_>>(), ["calm"], "the score has one layer");
    assert_eq!(state.live(), 1, "and one live layer voice");
    assert!(
        app.world().resource::<bevy_siren::SirenLog>().iter().any(|e| matches!(
            e, Entry::Stopped { cue, why, .. } if cue == "track.combat" && why == "layer removed")),
        "the stop says why: {:?}",
        app.world().resource::<bevy_siren::SirenLog>().iter().map(Entry::line).collect::<Vec<_>>()
    );
    let cues: Vec<String> =
        app.world_mut().query::<&Voice>().iter(app.world()).map(|v| v.cue.clone()).collect();
    assert_eq!(cues, ["track.calm"], "only the surviving stem is left");
}

/// **A layer retargeted to another cue plays the new one.** The unfixed tree kept the **old** sample and never started
/// the new: the score said one thing and the player heard another.
#[test]
fn a_layer_retargeted_to_another_cue_plays_the_new_one() {
    use bevy_siren::Entry;

    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    assert_eq!(app.world().resource::<MusicState>().live(), 2);

    // Point `calm` at the other track.
    app.world_mut().resource_mut::<MusicScore>().layers[0].cue = "track.combat".to_owned();
    run_for(&mut app, 0.6, 1.0 / 60.0);

    let state = app.world().resource::<MusicState>();
    assert_eq!(state.names().collect::<Vec<_>>(), ["calm", "combat"], "the layer kept its name");
    assert!(
        app.world().resource::<bevy_siren::SirenLog>().iter().any(|e| matches!(
            e, Entry::Stopped { cue, why, .. } if cue == "track.calm" && why == "layer retargeted")),
        "the old cue's voice says why it stopped"
    );
    let cues: Vec<String> =
        app.world_mut().query::<&Voice>().iter(app.world()).map(|v| v.cue.clone()).collect();
    assert!(cues.iter().all(|c| c == "track.combat"), "what the score says is what is playing: {cues:?}");
}

/// **A gain edit keeps a layer's voice** — the pair that stops the fix from restarting on every edit. That is what
/// `move_layers` moves, and a restart would be audible as a phrase starting over.
#[test]
fn a_layer_whose_gain_changed_keeps_its_voice() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    let voice = app.world().resource::<MusicState>().voice("calm").expect("the calm layer has a voice");

    app.world_mut().resource_mut::<MusicScore>().layers[0].gain_db = -12.0;
    run_for(&mut app, 0.3, 1.0 / 60.0);

    assert_eq!(
        app.world().resource::<MusicState>().voice("calm"),
        Some(voice),
        "the same entity is still the calm layer's voice"
    );
    let cues: Vec<String> =
        app.world_mut().query::<&Voice>().iter(app.world()).map(|v| v.cue.clone()).collect();
    assert!(cues.contains(&"track.calm".to_owned()), "and it still plays the same cue: {cues:?}");
}

/// **A play on the music bus cannot take a layer's voice, whatever priority the layer's cue authored.**
///
/// `Voice::unstealable`'s doc says *"music, stingers and dialogue are not stolen"*, and R4 in `docs/research/siren.md`
/// makes that a rule. The mechanism was `priority >= UNSTEALABLE_PRIORITY`, so a layer authored with the crate's
/// default `priority: 0` was **called** music and **treated as** a footstep: a `ui.click` on the same bus at
/// `max_voices: 1` stopped the stem (`stopped track.calm: stolen by ui.click`) and the score never heard about it.
/// A layer is a **stem**; the budget refuses the newcomer instead.
#[test]
fn a_one_shot_cannot_steal_a_music_layer() {
    // The layer's cue carries the crate's **default** priority, so `unstealable` is false by label: the protection this
    // test is about must come from the voice being a layer, not from the number the author wrote.
    let mut table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("track.calm")],
        bus: "Music".to_owned(),
        mode: Mode::Loop,
        ..Cue::new("track.calm")
    }]);
    table.insert(Cue {
        sounds: vec![SoundSlot::new("ui.click")],
        bus: "Music".to_owned(),
        ..Cue::new("ui.click")
    });
    let mut app = app(
        table,
        SirenSettings { max_voices: 1, ..SirenSettings::default() },
        sounds(&["track.calm", "ui.click"]),
        true,
    );
    app.insert_resource(MusicScore {
        layers: vec![MusicLayer::new("calm", "track.calm")],
        tempo_bpm: 120.0,
        beats_per_bar: 4.0,
    });
    app.update();
    app.update();
    let calm = app.world().resource::<MusicState>().voice("calm").expect("the calm layer has a voice");
    assert_eq!(
        app.world().resource::<CueTable>().get("track.calm").expect("cue").priority,
        0,
        "the layer's cue is not unstealable by label"
    );

    app.world_mut().resource_mut::<Messages<Play>>().write(Play::cue("ui.click"));
    app.update();

    let refused = refused(&mut app);
    assert!(refused.iter().any(|r| r.cue == "ui.click"), "the newcomer is refused: {refused:?}");
    assert_eq!(
        app.world().resource::<MusicState>().voice("calm"),
        Some(calm),
        "the stem is still playing, on the same entity"
    );
    let lines: Vec<String> = app.world().resource::<SirenLog>().recent(8).map(|e| e.line()).collect();
    assert!(
        !lines.iter().any(|l| l.contains("stolen by") && l.contains("track.calm")),
        "no line says the stem was stolen: {lines:?}"
    );
}

/// **A layer whose voice died for a reason the plugin did not ask for plays again**, at the layer's own gain.
///
/// Gregory §14.5.8.4: a stopped voice frees its resources, and a *logical* sound outlives its physical voice. The mirror
/// in `MusicState` was written once and never reconciled, so a voice that vanished left `voice()` naming a
/// **despawned** entity and the stem silent for the rest of the process.
#[test]
fn a_layer_whose_voice_died_is_restarted() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    let first = app.world().resource::<MusicState>().voice("calm").expect("the calm layer has a voice");

    // The voice dies without the music plugin asking: a stopped sample's owner despawns it.
    app.world_mut().entity_mut(first).despawn();
    app.update();
    app.update();

    let second = app.world().resource::<MusicState>().voice("calm");
    assert!(second.is_some(), "the layer has a voice again");
    assert_ne!(second, Some(first), "and it is a new one");
    assert_eq!(app.world().resource::<MusicState>().live(), 2, "both layers are live, which is the truth");
}

/// **A layer a game stopped with `StopCue` stays stopped.** The restart rule exists for a voice that died behind the
/// plugin's back, not for one the game asked for: restarting a stem a game stopped would be the crate overriding a
/// decision it was told to make.
#[test]
fn a_layer_the_game_stopped_stays_stopped() {
    let mut app = app_with(two_layer_score(), two_layer_table(), &["track.calm", "track.combat"]);
    app.update();
    app.update();
    assert!(app.world().resource::<MusicState>().voice("calm").is_some(), "it starts");

    let before = played(&mut app).len();
    app.world_mut().resource_mut::<Messages<StopCue>>().write(StopCue { cue: "track.calm".into() });
    run_for(&mut app, 3.0, 1.0 / 60.0);

    let started_after = played(&mut app);
    assert!(!started_after.iter().any(|p| p.cue == "track.calm"), "it is not restarted: {started_after:?}");
    let _ = before;
    let state = app.world().resource::<MusicState>();
    assert_eq!(state.voice("calm"), None, "the mirror no longer names a dead entity");
    assert_eq!(state.live(), 1, "only the layer that was not stopped is live");
    let calm = state.layer_db("calm").expect("the layer is still in the score");
    assert!(
        calm <= bevy_siren::model::SILENT_DB,
        "and its gain reads silent, not the last gain it played at: {calm}"
    );
}

/// **A layer whose first play was refused is asked for again once a voice frees**, and refused only once.
///
/// `awaiting` is the intent to start a layer. It was pushed and never cleared, so a layer refused by a full budget was
/// blocked for the life of the process — no retry, no second line. Retrying every frame would instead write one refusal
/// per frame into the panel's log, so the retry waits for the live count to fall below the count at the refusal.
#[test]
fn a_refused_layer_start_is_tried_again_when_room_frees() {
    let mut table = two_layer_table();
    table.insert(Cue {
        sounds: vec![SoundSlot::new("bed")],
        bus: "Music".to_owned(),
        mode: Mode::Loop,
        priority: 200,
        ..Cue::new("bed")
    });
    let mut app = app_with(MusicScore::default(), table, &["track.calm", "track.combat", "bed"]);
    app.insert_resource(SirenSettings { max_voices: 1, ..SirenSettings::default() });
    app.update();
    // A bed takes the one slot before the score arrives.
    app.world_mut().resource_mut::<Messages<Play>>().write(Play::cue("bed"));
    app.update();
    app.update();
    app.insert_resource(two_layer_score());
    run_for(&mut app, 0.2, 1.0 / 60.0);
    assert_eq!(
        app.world().resource::<MusicState>().live(),
        0,
        "the score's layers were refused: {:#?}",
        refused(&mut app)
    );

    app.world_mut().resource_mut::<Messages<StopCue>>().write(StopCue { cue: "bed".into() });
    run_for(&mut app, 2.0, 1.0 / 60.0);

    assert!(
        app.world().resource::<MusicState>().live() > 0,
        "a layer is live now that the bed freed its slot"
    );
}
