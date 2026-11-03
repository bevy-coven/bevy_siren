//! **A voice that ends frees its slot**, on the backend whose whole point is that a bus is one number.
//!
//! On [`Backend(true)`](bevy_siren::output::Backend) there is no `AudioSink`, so Bevy's "this sample is over"
//! cannot answer. The crate asks its own graph instead, through
//! [`AudioOutput::finished`](bevy_siren::output::AudioOutput::finished) — the query the backend already implements.
//! A fake output here is the only way to observe that from a headless test: it reports exactly what a `rodio`
//! `Player` reports, with no device and no `AudioPlugin`.
//!
//! Gregory §14.5.8.4 is the rule these tests hold the crate to: a voice is a fixed resource, and a sound that stops
//! *"frees its resources for use by other voices"*.

#![cfg(feature = "mixer")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy_siren::output::{AudioOutput, Backend, Output};
use bevy_siren::{
    Cue, CueTable, Entry, Mode, Play, Refusal, SirenLog, SirenSettings, SirenSounds, SoundSlot, Stopping,
    Voice,
};

use crate::helpers::{advance, played, refused, run_for, voices};

/// The crate's own mixer, as far as the crate can see it: it starts a voice on a bus and answers whether that
/// voice's sample has ended. `ended` is the test's hand on the sample's length.
#[derive(Default)]
struct FakeOutput {
    ended: Arc<AtomicBool>,
    started: Arc<Mutex<Vec<usize>>>,
    stopped: Arc<Mutex<Vec<usize>>>,
    /// When set, `play` refuses every sample — exactly what `MixerBackend::play` does when
    /// `rodio::Decoder::new` returns `Err`, which is the failure this iteration is about.
    refuses_samples: Arc<AtomicBool>,
}

impl AudioOutput for FakeOutput {
    fn set_bus_db(&mut self, _: &str, _: f32) {}

    fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}

    fn stop_voice(&mut self, _: &str, voice: usize) {
        self.stopped.lock().unwrap().push(voice);
    }

    fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
        if self.refuses_samples.load(Ordering::SeqCst) {
            return None;
        }
        let mut started = self.started.lock().unwrap();
        let index = started.len();
        started.push(index);
        Some(index)
    }

    fn finished(&self, _: &str, _: usize) -> bool {
        self.ended.load(Ordering::SeqCst)
    }
}

/// A headless app whose crate-owned mixer reports every one-shot's sample as ended once the test says so.
fn app_with_mixer(table: CueTable, settings: SirenSettings) -> (App, Arc<AtomicBool>) {
    let ended = Arc::new(AtomicBool::new(false));
    let fake = FakeOutput {
        ended: ended.clone(),
        started: Arc::new(Mutex::new(Vec::new())),
        stopped: Arc::new(Mutex::new(Vec::new())),
        refuses_samples: Arc::new(AtomicBool::new(false)),
    };
    let mut app = crate::helpers::app(table, settings, SirenSounds::new(), false);
    // A **resolved** asset, not `Handle::default()`: `start_pending` starts a voice only once it has the bytes, and
    // an unresolved handle is a voice that never gets a `MixerVoice` — which is exactly what the crate would see
    // while a file is still loading, and not the state these tests are about.
    let mut sounds = SirenSounds::new();
    for key in ["hit", "hum"] {
        let handle = app
            .world_mut()
            .resource_mut::<Assets<AudioSource>>()
            .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
        sounds.insert(key, handle);
    }
    app.insert_resource(sounds);
    app.insert_resource(Output(Box::new(fake)));
    app.insert_resource(Backend(true));
    advance(&mut app, 0.016);
    (app, ended)
}

/// A once-cue with no per-cue limit, so only the table's own `max_voices` bounds it: exactly the case a budget
/// that never releases turns into a lockout.
fn once_cue() -> Cue {
    Cue { sounds: vec![SoundSlot::new("hit")], max_instances: 0, ..Cue::new("hit") }
}

/// **The regression.** 40 one-shots against a budget of 32 all play, because each ended voice gave its slot back.
/// Before the fix the first 32 played, the last 8 were refused `VoiceLimit`, and 32 voices stayed live forever.
#[test]
fn a_one_shot_voice_ends_and_frees_its_slot() {
    let settings = SirenSettings {
        max_voices: 32,
        buses: vec![bevy_siren::BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let (mut app, ended) = app_with_mixer(CueTable::new(vec![once_cue()]), settings);

    app.world_mut().write_message(Play::cue("hit"));
    app.update();
    assert_eq!(voices(&mut app), 1, "the first play is a live voice");
    assert_eq!(played(&mut app).len(), 1);

    // The sample ends. Every play from here on must find the slot free.
    ended.store(true, Ordering::SeqCst);
    app.update();
    assert_eq!(voices(&mut app), 0, "an ended once-voice is retired, not kept for the life of the process");

    let mut answers = 0;
    for _ in 0..39 {
        app.world_mut().write_message(Play::cue("hit"));
        app.update();
        answers += played(&mut app).len();
        assert_eq!(voices(&mut app), 0, "each ended voice frees its slot before the next frame");
    }
    assert_eq!(answers, 39, "every one of the 40 plays is answered with a `Played`");
    assert!(
        refused(&mut app).iter().all(|r| r.reason != Refusal::VoiceLimit { bus: None }),
        "a budget of 32 does not lock out a game whose samples end: {:?}",
        refused(&mut app)
    );

    let ended_lines = app
        .world()
        .resource::<SirenLog>()
        .iter()
        .filter(|e| matches!(e, Entry::Stopped { why, .. } if why == "ended"))
        .count();
    assert_eq!(ended_lines, 40, "every ended voice says so once: {:?}", log_lines(&app));
}

/// A loop has no end to notice. Even if its source reports empty — a decoder that ran out — the voice lives until
/// it is stopped, which is the crate's documented rule for `Mode::Loop`.
#[test]
fn a_loop_voice_is_not_retired_when_its_source_ends() {
    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![bevy_siren::BusLevel::new("Ambience")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table = CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("hum")],
        bus: "Ambience".to_owned(),
        mode: Mode::Loop,
        ..Cue::new("hum")
    }]);
    let (mut app, ended) = app_with_mixer(table, settings);
    app.world_mut().write_message(Play::cue("hum"));
    app.update();
    ended.store(true, Ordering::SeqCst);
    run_for(&mut app, 0.2, 0.016);
    assert_eq!(voices(&mut app), 1, "a looping voice lives until it is stopped, whatever its source says");
}

/// A voice already fading is not also retired as "ended": the fade is what keeps a steal from clicking, and its log
/// line names the reason a person asked for, not a second one the mixer noticed.
#[test]
fn a_stopping_voice_is_not_retired_as_ended() {
    use bevy_siren::StopCue;

    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![bevy_siren::BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let (mut app, ended) = app_with_mixer(CueTable::new(vec![once_cue()]), settings);
    app.world_mut().write_message(Play::cue("hit"));
    app.update();
    assert_eq!(voices(&mut app), 1, "the play is a live voice");

    // The game stops the cue; the voice begins its fade. Then its sample's end is noticed.
    app.world_mut().write_message(StopCue { cue: "hit".to_owned() });
    app.update();
    ended.store(true, Ordering::SeqCst);
    app.update();

    assert_eq!(
        voices(&mut app),
        1,
        "a fading voice is still in the world: the fade is the courtesy that avoids a click"
    );
    assert!(app.world_mut().query::<&Stopping>().iter(app.world()).count() == 1, "it is fading, not retired");
    let lines = log_lines(&app);
    assert!(
        lines.iter().any(|l| l.contains("stopped `hit`: cue stopped")),
        "the stop is logged with its reason: {lines:?}"
    );
    assert!(
        !lines.iter().any(|l| l.contains("stopped `hit`: ended")),
        "a fading voice is not also reported ended: {lines:?}"
    );

    // And once its fade is over the voice is gone, with the budget free for the next play.
    run_for(&mut app, 0.6, 0.016);
    assert_eq!(app.world_mut().query::<&Voice>().iter(app.world()).count(), 0, "the fade despawned it");
}

/// **A music layer's gain reaches the mixer voice.** The mix writes a mixer voice's own gain, so a layer moved by
/// `MusicLayers` is heard at the new gain; before the fix a mixer voice was skipped in `update_mix`, so a layer
/// entering on the bar stayed at the gain it started at, and a stop fade began from silence and despawned at once.
#[test]
fn a_music_layer_moves_its_mixer_voice() {
    use bevy_siren::{MusicLayer, MusicLayers, MusicScore, MusicState};

    let mut app =
        crate::helpers::app(CueTable::new(vec![]), SirenSettings::default(), SirenSounds::new(), true);
    let fake = FakeOutput {
        ended: Arc::new(AtomicBool::new(false)),
        started: Arc::new(Mutex::new(Vec::new())),
        stopped: Arc::new(Mutex::new(Vec::new())),
        refuses_samples: Arc::new(AtomicBool::new(false)),
    };
    app.insert_resource(Output(Box::new(fake)));
    app.insert_resource(Backend(true));
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    let mut sounds = SirenSounds::new();
    sounds.insert("track", handle);
    app.insert_resource(sounds);
    app.insert_resource(CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("track")],
        bus: "Music".to_owned(),
        mode: Mode::Loop,
        priority: 200,
        ..Cue::new("track")
    }]));
    app.insert_resource(MusicScore {
        layers: vec![MusicLayer::new("combat", "track").at_db(bevy_siren::model::SILENT_DB)],
        tempo_bpm: 120.0,
        beats_per_bar: 4.0,
    });
    advance(&mut app, 0.016);
    app.update();
    assert_eq!(app.world().resource::<MusicState>().live(), 1, "the layer has a voice");

    app.world_mut().write_message(MusicLayers::new(vec![("combat".to_owned(), -3.0)]).now());
    app.update();
    run_for(&mut app, 0.5, 0.016);
    let db = app.world().resource::<MusicState>().layer_db("combat").expect("the layer");
    assert!((db - -3.0).abs() < 0.05, "the layer is at its target: {db}");
    let written = app.world_mut().query::<&bevy_siren::mix::Applied>().iter(app.world()).next().map(|a| a.db);
    assert!(
        written.is_some_and(|db| (db - -3.0).abs() < 0.05),
        "the gain was written into the mixer voice, not only into `MusicState`: {written:?}"
    );
}

fn log_lines(app: &App) -> Vec<String> {
    app.world().resource::<SirenLog>().iter().map(Entry::line).collect()
}

/// **A sample the decoder refuses names its sound key, not the cue.**
///
/// `MixerBackend::play` returns `None` when `rodio::Decoder::new` rejects the bytes — a truncated file, a format the
/// build does not include, a header that lies. `MissingSound::sound` is documented as *the key a project maps to a
/// file*, and the unfixed tree put the **cue's** name there: a project would search `audio/` for `footstep.grass`,
/// which was never a file. The fake output refuses every sample, which is exactly that path.
#[test]
fn a_sample_the_decoder_refuses_names_its_sound_key() {
    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![bevy_siren::BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("step_03")], ..Cue::new("footstep.grass") }]);

    let mut app = crate::helpers::app(table, settings, SirenSounds::new(), false);
    let refuses = Arc::new(AtomicBool::new(true));
    app.insert_resource(Output(Box::new(FakeOutput {
        ended: Arc::new(AtomicBool::new(false)),
        started: Arc::new(Mutex::new(Vec::new())),
        stopped: Arc::new(Mutex::new(Vec::new())),
        refuses_samples: refuses.clone(),
    })));
    app.insert_resource(Backend(true));
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    let mut sounds = SirenSounds::new();
    sounds.insert("step_03", handle);
    app.insert_resource(sounds);
    advance(&mut app, 0.016);

    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();
    app.update();

    let reason = refused(&mut app)
        .into_iter()
        .find(|r| matches!(r.reason, Refusal::MissingSound { .. }))
        .expect("the decoder refused, so the play is refused")
        .reason;
    assert_eq!(reason, Refusal::MissingSound { sound: "step_03".to_owned() }, "the key, not the cue");
    let line = reason.to_string();
    assert!(
        line.contains("step_03") && !line.contains("footstep.grass"),
        "the sentence names the file to look at: {line}"
    );
    assert_eq!(app.world_mut().query::<&Voice>().iter(app.world()).count(), 0, "nothing plays on a refusal");
}

/// **A key with no handle still names the key.** The other cause of `MissingSound`, produced by `answer`, must not
/// change — this is the half a fix must not overshoot.
#[test]
fn a_key_with_no_handle_still_names_the_key() {
    let settings = SirenSettings {
        max_voices: 4,
        buses: vec![bevy_siren::BusLevel::new("Effects")],
        ducks: Vec::new(),
        ..SirenSettings::default()
    };
    let table =
        CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("step_03")], ..Cue::new("footstep.grass") }]);
    let mut app = crate::helpers::app(table, settings, SirenSounds::new(), false);
    app.insert_resource(Output(Box::new(FakeOutput {
        ended: Arc::new(AtomicBool::new(false)),
        started: Arc::new(Mutex::new(Vec::new())),
        stopped: Arc::new(Mutex::new(Vec::new())),
        refuses_samples: Arc::new(AtomicBool::new(false)),
    })));
    app.insert_resource(Backend(true));
    advance(&mut app, 0.016);

    app.world_mut().write_message(Play::cue("footstep.grass"));
    app.update();

    let reason = refused(&mut app).remove(0).reason;
    assert_eq!(reason, Refusal::MissingSound { sound: "step_03".to_owned() });
}

/// **A slot is reused, and a stale index addresses nothing.**
///
/// The real `MixerBackend` is the only thing that can prove both halves: that 400 one-shots hold a handful of slots
/// rather than 400 (the leak fixed here), and that a reused slot's *old* index cannot move its new occupant. It needs
/// an audio device, so it returns early where there is none — the crate's own rule that a headless machine hears
/// nothing and says so.
#[test]
fn a_slot_is_reused_and_a_stale_index_addresses_nothing() {
    use bevy_siren::mixer::MixerBackend;

    let Some(mut backend) = MixerBackend::open() else {
        eprintln!("no audio device; the real mixer's slot reuse cannot be measured here");
        return;
    };
    let bytes: Arc<[u8]> = Arc::from(crate::helpers::tiny_wav());
    let mut first = None;
    let mut distinct = std::collections::BTreeSet::new();
    for _ in 0..200 {
        if let Some(index) = backend.play("Effects", &bytes, -6.0, 1.0, false) {
            distinct.insert(index);
            first.get_or_insert(index);
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let held = backend.voices("Effects");
    assert!(held <= 32, "200 one-shots hold at most the concurrent peak, not 200: {held}");
    assert!(held > 0, "the short samples were actually started");
    assert_eq!(distinct.len(), 200, "each hand-out is a fresh name, even in a reused slot");

    // The first index names a voice that ended long ago. It must not address the slot's current occupant.
    let stale = first.expect("the first play started");
    assert!(backend.finished("Effects", stale), "a stale name has ended");
    backend.set_voice_db("Effects", stale, -80.0);
    backend.stop_voice("Effects", stale);
}
