// scratch: README.md's "Quick start" block, verbatim as it stood at 0a0e359 (deleted before the commit; a copy is kept in evidence/before).
use bevy::prelude::*;
use bevy_siren::*;

fn main() {
    App::new().add_plugins((DefaultPlugins, SirenPlugin)).add_systems(Startup, setup).run();
}

fn setup(mut cue_table: ResMut<CueTable>, mut sounds: ResMut<SirenSounds>, assets: Res<AssetServer>) {
    // A cue with two variants: the picker never plays the same one twice in a row.
    cue_table.insert(Cue {
        sounds: vec![SoundSlot::new("step_01"), SoundSlot::new("step_02")],
        ..Cue::new("footstep.grass")
    });
    sounds.load_table(&assets, &cue_table, "audio");
}

fn walk(mut play: MessageWriter<Play>) {
    play.write(Play::cue("footstep.grass"));
}

fn hear(mut played: MessageReader<Played>, mut log: ResMut<SirenLog>) {
    for answer in played.read() {
        // Which recording the player heard, and why it was chosen.
        log.push(Entry::Played {
            at_s: 0.0,
            cue: answer.cue.clone(),
            sound: answer.sound.clone(),
            bus: answer.bus.clone(),
            gain_db: answer.gain_db,
        });
    }
}
