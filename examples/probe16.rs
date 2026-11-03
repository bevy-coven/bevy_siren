//! Throwaway (deleted before commit): untested combinations, one run.
use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy_siren::*;

fn build(table: CueTable, settings: SirenSettings, keys: &[&str]) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)))
        .add_plugins(SirenPlugin)
        .insert_resource(settings)
        .insert_resource(table);
    let mut sounds = SirenSounds::new();
    for k in keys {
        let h = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource { bytes: std::sync::Arc::from(vec![0u8; 4]) });
        sounds.insert(*k, h);
    }
    app.insert_resource(sounds);
    app.update();
    app
}

fn ask(app: &mut App, play: Play) -> (Vec<String>, Vec<String>) {
    app.world_mut().write_message(play);
    app.update();
    let p = app
        .world_mut()
        .resource_mut::<Messages<Played>>()
        .drain()
        .map(|x| format!("{}@{:.2}", x.sound, x.gain_db))
        .collect();
    let r = app
        .world_mut()
        .resource_mut::<Messages<Refused>>()
        .drain()
        .map(|x| format!("{:?}", x.reason))
        .collect();
    (p, r)
}

fn main() {
    // 1. A cue whose mode is Loop, stopped by StopCue, then played again — is the history right?
    // 2. `Repeat::AvoidRepeat { avoid: 0 }` — a cue that says "always avoid 0", i.e. no constraint.
    // 3. A cue with one variant and `AvoidRepeat { avoid: 4 }` — refuse the last 4 of 1.
    // 4. A cue whose pitch range is a single value (low == high).
    // 5. `max_instances` on a cue whose bus cap is smaller.
    let table = CueTable::new(vec![
        Cue {
            sounds: vec![SoundSlot::new("a")],
            repeat: Repeat::AvoidRepeat { avoid: 0 },
            ..Cue::new("zero")
        },
        Cue {
            sounds: vec![SoundSlot::new("a")],
            repeat: Repeat::AvoidRepeat { avoid: 4 },
            ..Cue::new("wide")
        },
        Cue {
            sounds: vec![SoundSlot::new("a")],
            pitch: PitchRange { low: 2.0, high: 2.0 },
            ..Cue::new("exact")
        },
    ]);
    let mut app = build(table, SirenSettings::default(), &["a"]);

    let (p, r) = ask(&mut app, Play::cue("zero"));
    println!("avoid 0:      {p:?} {r:?}  (0 means 'no constraint', so repeats are fine)");

    let mut seen = Vec::new();
    for _ in 0..6 {
        let (p, _) = ask(&mut app, Play::cue("wide"));
        seen.extend(p);
    }
    println!("avoid 4 of 1: {seen:?}  (only one variant, so it must still play every time)");

    let (p, _) = ask(&mut app, Play::cue("exact"));
    println!("pitch 2..2:   {p:?}  (pitch is not in the line, so re-read Played below)");
}
