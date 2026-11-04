use bevy::prelude::*;
use bevy_siren::*;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SirenPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, (walk, report))
        .run();
}

fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
    // A cue with two variants: the picker never plays the same one twice in a row.
    // A sound key is a file's name under `assets/audio/`, extension included.
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01.ogg"), SoundSlot::new("step_02.ogg")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

// A footstep every half second.
fn walk(mut play: MessageWriter<Play>, time: Res<Time>, mut elapsed: Local<f32>) {
    *elapsed += time.delta_secs();
    if *elapsed >= 0.5 {
        *elapsed -= 0.5;
        play.write(Play::cue("footstep.grass"));
    }
}

// What the crate decided, once each, in its own words: which recording played, or why nothing did.
fn report(log: Res<SirenLog>, mut seen: Local<u64>) {
    for entry in log.since(*seen) {
        info!("{}", entry.line());
    }
    *seen = log.pushed();
}
