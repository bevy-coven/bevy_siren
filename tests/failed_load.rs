//! **A file that failed to load is refused, with the cause — not answered `Played` and played in silence.**
//!
//! Every other refusal in this crate is decided without a file: a cue the table does not have, a key with no handle, a bus that is not in the mix. A handle whose **file failed to load** — a typo in a
//! name, a file nobody delivered — was none of them. The handle exists, the asset server knows the load failed and says why (`LoadState::Failed`), and the crate waited for the bytes for ever: `Played`
//! was written, the voice sat in the budget, the log showed a healthy *played* and *stolen by …*, and nothing sounded (`docs/loop/runs/2026-11-04-116/`).
//!
//! These tests use the **real `AssetServer`** over the fixtures in `tests/fixtures`, with a loader of their own for `.ogg` that **waits at a gate the test opens** and then succeeds or fails as told, so the order of
//! events is the test's and not a race between a delay and the next frame. A default (Uuid) handle — what every other test uses — is a handle the server has no record of and answers `None` for, which is why none of them could build this.
//!
//! **Bevy's path is forced in every build by removing the `Output`**, as `ogg_loops` does; the crate's own mixer has its own test below, behind the `mixer` feature.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, AssetPlugin, LoadContext, LoadState};
use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy_siren::output::Output;
use bevy_siren::{
    Cue, CueTable, Mode, MusicLayer, MusicPlugin, MusicScore, MusicState, Play, Refusal, SirenLog, SirenMix,
    SirenPlugin, SirenSounds, SoundSlot,
};

use crate::helpers::{played, refused, voices};

/// A gate a loader waits at until a test opens it, so **the order of events is the test's** and not a race between a sleep and the next frame. The loader gives up waiting when the test that holds the other end is gone
/// (a panic) or after ten seconds, so a failed test does not keep a loader thread from the tests after it.
#[derive(Clone, Default)]
struct Gate(Arc<AtomicBool>);

impl Gate {
    fn open(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn wait(&self) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.0.load(Ordering::SeqCst) && Arc::strong_count(&self.0) > 1 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// A loader for `.ogg` that waits at its `gate` and then **succeeds, or fails when the path ends with `fails_on`** (`Some("")` fails every file, `None` none) **or once `broken` is set** — an edited file that no longer loads.
#[derive(TypePath)]
struct Gated {
    gate: Gate,
    fails_on: Option<&'static str>,
    broken: Arc<AtomicBool>,
}

impl Gated {
    fn new(gate: &Gate, fails_on: Option<&'static str>) -> Self {
        Self { gate: gate.clone(), fails_on, broken: Arc::default() }
    }
}

impl AssetLoader for Gated {
    type Asset = AudioSource;
    type Settings = ();
    type Error = std::io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<AudioSource, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        self.gate.wait();
        let path = load_context.path().to_string();
        if self.broken.load(Ordering::SeqCst) || self.fails_on.is_some_and(|name| path.ends_with(name)) {
            return Err(std::io::Error::other("this loader was told to fail"));
        }
        Ok(AudioSource { bytes: bytes.into() })
    }

    fn extensions(&self) -> &[&str] {
        &["ogg"]
    }
}

/// A headless app over `tests/fixtures` with the real asset server, the loader above, and `table`. **No `Output`**, so Bevy's path owns every voice.
fn app_with(loader: Gated, table: CueTable) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin { file_path: "tests/fixtures".into(), ..default() }))
        .init_asset::<AudioSource>()
        .register_asset_loader(loader)
        .add_plugins((SirenPlugin, MusicPlugin));
    app.insert_resource(table);
    app.world_mut().remove_resource::<Output>();
    app
}

/// The cue `"click"` playing the file `tone.ogg`, one at a time (the default).
fn click() -> CueTable {
    CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("tone.ogg")], ..Cue::new("click") }])
}

/// Asks the asset server for `file` and maps the key `file` to the handle, as a game does at startup.
fn load(app: &mut App, file: &str) -> Handle<AudioSource> {
    let handle = app.world().resource::<AssetServer>().load::<AudioSource>(file.to_owned());
    app.world_mut().resource_mut::<SirenSounds>().insert(file, handle.clone());
    handle
}

fn state(app: &App, handle: &Handle<AudioSource>) -> Option<LoadState> {
    app.world().resource::<AssetServer>().get_load_state(handle.id())
}

fn failed(app: &App, handle: &Handle<AudioSource>) -> bool {
    matches!(state(app, handle), Some(LoadState::Failed(_)))
}

/// Runs frames a few milliseconds apart — the loader works on another thread — until `done`, or fails after five seconds.
fn until(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        if done(app) {
            return;
        }
        assert!(Instant::now() < deadline, "gave up waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn lines(app: &App) -> Vec<String> {
    app.world().resource::<SirenLog>().iter().map(|entry| entry.line()).collect()
}

/// **A play whose file has already failed is refused, naming the file and the cause, and plays nothing.** The unchanged tree answered `Played` for every play of a typo'd file (six of six, real server,
/// `evidence/before/failed-load-probe-on-the-unchanged-tree.txt`) and the log said *played* and *stolen by click (cue)*.
#[test]
fn a_play_of_a_file_that_already_failed_to_load_is_refused_with_the_cause() {
    let gate = Gate::default();
    gate.open();
    let mut app = app_with(Gated::new(&gate, Some("")), click());
    let handle = load(&mut app, "tone.ogg");
    until(&mut app, "the load to fail", |app| failed(app, &handle));

    app.world_mut().write_message(Play::cue("click"));
    app.update();

    assert!(played(&mut app).is_empty(), "nothing plays on a refusal, so nothing is announced as played");
    let answers = refused(&mut app);
    assert_eq!(answers.len(), 1, "one play, one answer: {answers:?}");
    assert_eq!(
        answers[0].reason,
        Refusal::MissingSound { sound: "tone.ogg".to_owned() },
        "the key to fix, not the cue"
    );
    assert_eq!(voices(&mut app), 0, "no voice was spawned to wait for bytes that will never come");
    assert_eq!(
        app.world().resource::<SirenMix>().voices_on("Effects"),
        0,
        "and nothing is counted against the budget"
    );

    let log = lines(&app);
    assert!(
        log.iter().any(|l| l.contains("refused")
            && l.contains("tone.ogg")
            && l.contains("failed to load")
            && l.contains("this loader was told to fail")),
        "the log names the sound, says the file failed to load and carries the asset server's own cause: {log:?}"
    );
    assert!(!log.iter().any(|l| l.contains(" played ")), "and never says it played: {log:?}");
}

/// **A voice spawned while the load was in flight, whose load then fails, is refused in the frame the server says so — and freed.** The play could not know, so `Played` was already written (the same
/// two answers `start_pending` gives bytes the decoder refuses); the unchanged tree kept the voice for ever.
#[test]
fn a_voice_whose_load_fails_after_it_was_played_is_refused_in_that_frame_and_freed() {
    let gate = Gate::default();
    let mut app = app_with(Gated::new(&gate, Some("")), click());
    let handle = load(&mut app, "tone.ogg");

    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(
        state(&app, &handle).map(|s| s.is_loading()),
        Some(true),
        "the premise: the load is still in flight when the play is asked — the loader is held at its gate, so this is certain and not a race"
    );
    assert_eq!(
        played(&mut app).len(),
        1,
        "a play whose file is still loading is played — it waits for its bytes"
    );
    assert!(refused(&mut app).is_empty());
    assert_eq!(voices(&mut app), 1);

    // Let the load finish — it fails — and count frames until the server reports it; the refusal must arrive in the very frame it does.
    gate.open();
    let mut frames = 0;
    let (mut failed_at, mut refused_at) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(5);
    while refused_at.is_none() {
        app.update();
        frames += 1;
        if failed_at.is_none() && failed(&app, &handle) {
            failed_at = Some(frames);
        }
        if !refused(&mut app).is_empty() {
            refused_at = Some(frames);
        }
        assert!(
            Instant::now() < deadline,
            "the voice was never refused: it waits for ever for a file the server says failed (state {:?})",
            state(&app, &handle)
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(refused_at, failed_at, "refused in the frame the load state became Failed");
    assert_eq!(voices(&mut app), 0, "the voice is gone");
    assert_eq!(app.world().resource::<SirenMix>().voices_on("Effects"), 0, "and its slot is free");

    let log = lines(&app);
    let (said_played, said_refused) = (
        log.iter().position(|l| l.contains(" played ")),
        log.iter()
            .position(|l| l.contains("refused") && l.contains("failed to load") && l.contains("tone.ogg")),
    );
    assert!(
        said_played.is_some() && said_refused > said_played,
        "played, then refused with the cause: {log:?}"
    );
}

/// **The control: a load still in flight is not refused.** It waits, as it always did, and what arrives is played; a file that loaded is untouched.
#[test]
fn a_load_still_in_flight_is_not_refused_and_a_loaded_file_plays() {
    let gate = Gate::default();
    let mut app = app_with(Gated::new(&gate, None), click());
    let handle = load(&mut app, "tone.ogg");

    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(played(&mut app).len(), 1);
    // Held at the gate the load is certainly in flight: frames pass and the voice is neither refused nor freed.
    for _ in 0..10 {
        app.update();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(refused(&mut app).is_empty(), "a load still in flight is not refused");
    assert_eq!(voices(&mut app), 1, "the voice waits for its bytes");
    gate.open();
    let mut answers = Vec::new();
    until(&mut app, "the file to load", |app| {
        answers.extend(refused(app));
        app.world().resource::<Assets<AudioSource>>().get(&handle).is_some()
    });
    app.update();
    answers.extend(refused(&mut app));
    assert!(
        answers.is_empty(),
        "nothing refused while the file was loading, nor when it arrived: {answers:?}"
    );
    assert_eq!(voices(&mut app), 1, "the voice that waited is still the voice");

    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(played(&mut app).len(), 1, "a loaded file plays");
    assert!(refused(&mut app).is_empty());
}

/// **A file whose reload failed, but whose bytes are loaded, still plays.** A hot reload of an edited file that no longer loads leaves the server's state `Failed` — and the **previous** bytes in `Assets`, because a failed
/// reload removes nothing. The sound *can* be played, so it is: refusing it would silence a cue the game could still sound, for a file the developer is half-way through saving. (`SirenSounds::failed` still lists it.)
#[test]
fn a_file_whose_reload_failed_but_whose_bytes_are_there_still_plays() {
    let gate = Gate::default();
    gate.open();
    let loader = Gated::new(&gate, None);
    let broken = loader.broken.clone();
    let mut app = app_with(loader, click());
    let handle = load(&mut app, "tone.ogg");
    until(&mut app, "the file to load", |app| {
        app.world().resource::<Assets<AudioSource>>().get(&handle).is_some()
    });

    broken.store(true, Ordering::SeqCst);
    app.world().resource::<AssetServer>().reload("tone.ogg");
    until(&mut app, "the reload to fail", |app| failed(app, &handle));
    assert!(
        app.world().resource::<Assets<AudioSource>>().get(&handle).is_some(),
        "the premise: a failed reload leaves the loaded bytes in place"
    );

    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(played(&mut app).len(), 1, "the bytes are there, so the sound plays");
    assert!(refused(&mut app).is_empty(), "and nothing is refused");
    for _ in 0..3 {
        app.update();
    }
    assert!(refused(&mut app).is_empty(), "nor is the voice, in the frames after");
    assert_eq!(voices(&mut app), 1);
}

/// **The same on the crate's own mixer**: a failed load never reaches `AudioOutput::play`, the voice is refused and gone. The check is upstream of both backends.
#[cfg(feature = "mixer")]
#[test]
fn on_the_crates_own_mixer_a_failed_load_never_reaches_the_output() {
    use std::sync::atomic::AtomicUsize;

    use bevy_siren::output::{AudioOutput, Backend};

    #[derive(Clone, Default)]
    struct Counting(Arc<AtomicUsize>);

    impl AudioOutput for Counting {
        fn set_bus_db(&mut self, _: &str, _: f32) {}
        fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}
        fn stop_voice(&mut self, _: &str, _: usize) {}
        fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
            Some(self.0.fetch_add(1, Ordering::SeqCst))
        }
        fn finished(&self, _: &str, _: usize) -> bool {
            false
        }
    }

    let gate = Gate::default();
    let mut app = app_with(Gated::new(&gate, Some("")), click());
    let counting = Counting::default();
    app.insert_resource(Output(Box::new(counting.clone())));
    app.insert_resource(Backend(true));
    let handle = load(&mut app, "tone.ogg");

    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(played(&mut app).len(), 1);
    gate.open();
    until(&mut app, "the load to fail", |app| failed(app, &handle));
    app.update();

    let answers = refused(&mut app);
    assert_eq!(answers.len(), 1, "one refusal for the one failed load: {answers:?}");
    assert_eq!(answers[0].reason, Refusal::MissingSound { sound: "tone.ogg".to_owned() });
    assert_eq!(voices(&mut app), 0);
    assert_eq!(
        counting.0.load(Ordering::SeqCst),
        0,
        "the output was never asked to play bytes that never came"
    );
}

/// **A music layer whose stem failed is refused with the cause, the other layer plays, and the log is not written every frame for the dead one.** The layer is a virtual voice
/// (`MusicState::blocked`), the same state a budget refusal leaves, until a voice frees.
#[test]
fn a_layer_whose_stem_failed_is_refused_once_the_other_plays_and_the_log_stays_quiet() {
    let layer = |name: &str, file: &str| Cue {
        sounds: vec![SoundSlot::new(file)],
        bus: "Music".to_owned(),
        mode: Mode::Loop,
        priority: 200,
        ..Cue::new(name)
    };
    let table = CueTable::new(vec![layer("track.calm", "tone.ogg"), layer("track.combat", "tone-441.ogg")]);
    let gate = Gate::default();
    let mut app = app_with(Gated::new(&gate, Some("tone-441.ogg")), table);
    let (calm, combat) = (load(&mut app, "tone.ogg"), load(&mut app, "tone-441.ogg"));
    app.insert_resource(MusicScore {
        layers: vec![MusicLayer::new("calm", "track.calm"), MusicLayer::new("combat", "track.combat")],
        tempo_bpm: 120.0,
        beats_per_bar: 4.0,
    });
    // Both loads are held at the gate: the layers start and wait for their bytes (so each voice is played and then, for the dead stem, refused in flight), and only then does the gate open.
    for _ in 0..3 {
        app.update();
    }
    assert_eq!(voices(&mut app), 2, "both layers' voices are waiting for their bytes");
    gate.open();

    until(&mut app, "the stem to fail", |app| failed(app, &combat));
    let mut answers = refused(&mut app);
    // A few more frames for the layer's retry, then many with nothing changing.
    for _ in 0..4 {
        app.update();
        std::thread::sleep(Duration::from_millis(5));
    }
    answers.extend(refused(&mut app));
    let before_quiet = answers.len();
    for _ in 0..60 {
        app.update();
    }
    answers.extend(refused(&mut app));

    assert!(
        answers.iter().all(|r| r.reason == Refusal::MissingSound { sound: "tone-441.ogg".to_owned() }),
        "every refusal names the stem that failed: {answers:?}"
    );
    assert!(!answers.is_empty(), "the dead stem was refused");
    assert_eq!(
        answers.len(),
        before_quiet,
        "and 60 further frames wrote nothing more: the layer waits for a voice to free, it does not retry every frame ({answers:?})"
    );
    let state_of_music = app.world().resource::<MusicState>();
    assert_eq!(state_of_music.live(), 1, "the other layer is live");
    assert!(
        state_of_music.blocked("combat").is_some(),
        "the dead layer is a virtual voice, and the state says so"
    );
    assert!(state(&app, &calm).is_some_and(|s| s.is_loaded()), "the calm stem loaded");
    assert!(
        lines(&app).iter().any(|l| l.contains("failed to load") && l.contains("tone-441.ogg")),
        "and the log says which file: {:?}",
        lines(&app)
    );
}
