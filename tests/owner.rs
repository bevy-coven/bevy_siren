//! **A voice belongs to its owner without being its child.**
//!
//! `Play::on(entity)` is documented as *"the sound belongs to `entity`"*, `Play::at(position)` as *"the sound is at `position` in the world"*, and `docs/guide/buses.md` as *"a sound `Play::on` hung from an entity stops when that entity is
//! despawned, and says so"*. It was implemented as one line — `ChildOf(owner)` on the voice — and a child is a hierarchy node: it inherits its parent's transform, it is made through a hook that queues a command on the parent, and it is despawned with it.
//! Run as a game would run them (`docs/loop/runs/2026-11-04-119/evidence/before/`), the states an owner can be in gave four defects: **a panic** in Bevy's command handler for an owner that is allocated and not yet spawned (the state every
//! `Commands::spawn` returns before its command applies); a sound **at the owner's transform composed with `at`** — 12 dB quiet for an owner at the place, and 52 dB over the sample for an owner with no `Transform`, which Bevy never propagates;
//! **a loop that lives for ever** on an owner that was already gone; and **a false *owner despawned* line** when the game despawned the voice itself.
//!
//! Ousterhout (*A Philosophy of Software Design*, §10.3) on defining errors out of existence — an owner not spawned yet has not begun, one that is gone is already over — Hunt and Thomas (*The Pragmatic Programmer*, tip 38) on crashing only for what was
//! supposed to be impossible, and Gregory (*Game Engine Architecture*, §14.4) on a source's single position for level and pan.
//!
//! Every test here runs on Bevy's path (the `Output` is removed) except the last, which gives the crate's own mixer a fake output.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::audio::{AudioSource, SpatialListener};
use bevy::prelude::*;
use bevy::transform::TransformPlugin;
use bevy_siren::model::{backend_spatial_gain, gain_to_db};
use bevy_siren::output::{AudioOutput, Backend, Output};
use bevy_siren::{
    Compensation, Cue, CueTable, Entry, Mode, Play, SirenLog, SirenMix, SirenSettings, SirenSounds,
    SoundSlot, Voice, VoiceOwner,
};

use crate::helpers::{app, played, sounds, voices};

/// The world position a sound is asked to be at.
const WORLD: Vec3 = Vec3::new(100.0, 0.0, 0.0);

fn engine(spatial: bool) -> CueTable {
    CueTable::new(vec![Cue {
        sounds: vec![SoundSlot::new("engine")],
        mode: Mode::Loop,
        spatial,
        ..Cue::new("engine")
    }])
}

/// A headless app on Bevy's path with Bevy's transform propagation, so a voice's `GlobalTransform` is what its panner would read, and a listener at the origin whose ears are 0.2 m apart.
fn world_app(spatial: bool) -> App {
    let mut app = app(engine(spatial), SirenSettings::default(), sounds(&["engine"]), false);
    app.world_mut().remove_resource::<Output>();
    app.add_plugins(TransformPlugin);
    app.world_mut().spawn((Transform::default(), GlobalTransform::default(), SpatialListener::new(0.2)));
    app
}

fn frames(app: &mut App, n: usize) {
    for _ in 0..n {
        app.update();
    }
}

fn lines(app: &App) -> Vec<String> {
    app.world().resource::<SirenLog>().iter().map(Entry::line).collect()
}

fn said(app: &App, what: &str) -> usize {
    lines(app).iter().filter(|line| line.contains(what)).count()
}

/// **An owner that is allocated and not yet spawned is an ordinary state, not a crash.** It is what `Commands::spawn` hands back before its command applies — a game that spawns a projectile and plays its sound in one system, unordered against the crate's,
/// meets it — and `world.entity_allocator().alloc()` builds it deterministically. The unchanged tree panicked in Bevy's command handler (*"Entity not yet spawned"*) in the first frame: the relationship hook queued a command on an entity that did not exist.
#[test]
fn an_owner_that_is_not_spawned_yet_does_not_crash_the_game() {
    let mut app = world_app(false);
    let owner = app.world().entity_allocator().alloc();
    app.world_mut().write_message(Play::cue("engine").on(owner));
    app.update();
    assert_eq!(voices(&mut app), 1, "the play was answered and its voice waits for its owner");

    app.world_mut()
        .spawn_at(owner, Transform::default())
        .expect("the reserved id takes its entity, as its queued command would");
    frames(&mut app, 3);
    assert_eq!(voices(&mut app), 1, "an owner that exists keeps its voice");
    assert_eq!(said(&app, "owner despawned"), 0, "and nothing says it was lost: {:?}", lines(&app));

    app.world_mut().despawn(owner);
    app.update();
    assert_eq!(voices(&mut app), 0, "the voice ends with its owner");
    assert_eq!(said(&app, "`engine`: owner despawned"), 1, "and says so once: {:?}", lines(&app));
}

/// **An owner that is already gone ends the sound in the frame it began, and says so.** The unchanged tree left a voice with no owner (Bevy warns and removes the relationship) — for a loop, a sound that lives for ever, counted on its bus,
/// with a log that says only *played*.
#[test]
fn a_sound_on_an_owner_that_is_already_gone_ends_in_its_frame_and_says_so() {
    let mut app = world_app(false);
    let owner = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(owner);
    app.world_mut().write_message(Play::cue("engine").on(owner));
    frames(&mut app, 3);

    assert_eq!(voices(&mut app), 0, "a loop on an owner that is gone does not live for ever");
    assert_eq!(
        app.world().resource::<SirenMix>().voices_on("Effects"),
        0,
        "and holds no slot against the bus"
    );
    assert_eq!(said(&app, " played `engine`"), 1, "the play was answered: {:?}", lines(&app));
    assert_eq!(
        said(&app, "`engine`: owner despawned"),
        1,
        "and the sound's end has its reason: {:?}",
        lines(&app)
    );
}

/// What the player hears of a placed voice, in dB re the sample: the crate's own gain and the compensation it wrote, then the backend's panner at the **voice's real position** — which is what Bevy reads (`GlobalTransform`).
fn heard_db(app: &App, voice: Entity, gain_db: f32) -> f32 {
    let compensation =
        app.world().get::<Compensation>(voice).expect("a placed voice carries its compensation").db;
    let at = app
        .world()
        .get::<GlobalTransform>(voice)
        .expect("a placed voice has a global transform")
        .translation();
    let ears = SpatialListener::new(0.2);
    let panner = backend_spatial_gain(at.into(), ears.left_ear_offset.into(), ears.right_ear_offset.into());
    gain_db + compensation + gain_to_db(panner)
}

/// **`at` is a world position, whatever the owner is.** `Play::at` says *"in the world"* and `Play`'s own example is `.on(foot).at(..)`; the unchanged tree composed the owner's transform with it — (200, 0, 0) for an owner at (100, 0, 0), and the listener's own
/// position for an owner with no `Transform` (Bevy never propagates a child of a parent that has none). The crate had computed its level and its panner compensation for `at`, so the player heard 12 dB less than the law — or **52 dB over the sample**.
/// Each arrangement is checked for the position, and for the level the player would hear against the law at `at`.
#[test]
fn a_sound_is_where_it_was_asked_to_be_whatever_its_owner_is() {
    type Owner = fn(&mut App) -> Entity;
    let arrangements: [(&str, Owner); 3] = [
        ("an owner at the place", |app| app.world_mut().spawn(Transform::from_translation(WORLD)).id()),
        ("an owner elsewhere", |app| app.world_mut().spawn(Transform::from_xyz(-50.0, 7.0, 3.0)).id()),
        ("an owner with no Transform", |app| app.world_mut().spawn_empty().id()),
    ];
    for (what, make_owner) in arrangements {
        let mut app = world_app(true);
        let owner = make_owner(&mut app);
        frames(&mut app, 2);
        app.world_mut().write_message(Play::cue("engine").on(owner).at(WORLD));
        frames(&mut app, 3);

        let answer = played(&mut app).remove(0);
        let at = app
            .world()
            .get::<GlobalTransform>(answer.entity)
            .expect("a placed voice has a global transform")
            .translation();
        assert!((at - WORLD).length() < 1e-3, "{what}: the voice is at {at}, asked for {WORLD}");
        let heard = heard_db(&app, answer.entity, answer.gain_db);
        assert!(
            (heard - answer.gain_db).abs() < 0.01,
            "{what}: the player hears {heard:.2} dB where the crate's law at {WORLD} says {:.2} dB",
            answer.gain_db
        );
    }
}

/// **A placed sound follows its owner** — what the hierarchy gave by accident and the crate documents (*"Bevy's path stays the default for moving emitters"*). The offset is fixed in the owner's space, so a translated owner carries the sound
/// with it and a turned one turns the offset: an owner at (100, 0, 0) moved by (10, 0, 0) puts the voice at (110, 0, 0), and an owner at the origin turned 90° about Y puts an offset of (1, 0, 0) at (0, 0, −1).
#[test]
fn a_placed_sound_follows_its_owner() {
    let mut app = world_app(true);
    let owner = app.world_mut().spawn(Transform::from_translation(WORLD)).id();
    frames(&mut app, 2);
    app.world_mut().write_message(Play::cue("engine").on(owner).at(WORLD));
    frames(&mut app, 2);
    let voice = played(&mut app).remove(0).entity;

    app.world_mut().entity_mut(owner).get_mut::<Transform>().expect("the owner's transform").translation =
        WORLD + Vec3::new(10.0, 0.0, 0.0);
    frames(&mut app, 3);
    let at = app.world().get::<GlobalTransform>(voice).expect("the voice's global transform").translation();
    assert!(
        (at - (WORLD + Vec3::new(10.0, 0.0, 0.0))).length() < 1e-3,
        "the owner moved by (10, 0, 0) and the voice is at {at}"
    );

    let mut app = world_app(true);
    let owner = app.world_mut().spawn(Transform::default()).id();
    frames(&mut app, 2);
    app.world_mut().write_message(Play::cue("engine").on(owner).at(Vec3::X));
    frames(&mut app, 2);
    let voice = played(&mut app).remove(0).entity;
    app.world_mut().entity_mut(owner).get_mut::<Transform>().expect("the owner's transform").rotation =
        Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    frames(&mut app, 3);
    let at = app.world().get::<GlobalTransform>(voice).expect("the voice's global transform").translation();
    assert!(
        (at - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-3,
        "the owner turned 90° about Y and an offset of (1, 0, 0) is at {at}"
    );
}

/// **A game that despawns a voice it was told about is not blamed on the owner.** `docs/guide/buses.md`: *"A game that despawns a voice it never parented gets no line"* — and one that despawns an owned voice while its owner lives
/// got **`stopped … owner despawned`**, a false line, because the crate read `ChildOf` as *"the owner did it"*.
#[test]
fn a_game_that_despawns_a_voice_is_not_told_the_owner_did() {
    let mut app = world_app(false);
    let owner = app.world_mut().spawn_empty().id();
    frames(&mut app, 1);
    app.world_mut().write_message(Play::cue("engine").on(owner));
    frames(&mut app, 2);
    let voice =
        app.world_mut().query_filtered::<Entity, With<Voice>>().iter(app.world()).next().expect("the voice");

    app.world_mut().despawn(voice);
    frames(&mut app, 3);
    assert_eq!(voices(&mut app), 0, "the game's despawn took the voice");
    assert_eq!(
        said(&app, "owner despawned"),
        0,
        "the owner is alive, and the log does not say it despawned: {:?}",
        lines(&app)
    );
}

/// A backend that starts every sample and counts them: the crate's own mixer, as far as a headless test can build it.
#[derive(Clone, Default)]
struct Counting {
    started: Arc<AtomicUsize>,
}

impl AudioOutput for Counting {
    fn set_bus_db(&mut self, _: &str, _: f32) {}
    fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}
    fn stop_voice(&mut self, _: &str, _: usize) {}
    fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
        Some(self.started.fetch_add(1, Ordering::SeqCst))
    }
    fn finished(&self, _: &str, _: usize) -> bool {
        false
    }
}

/// **On the crate's own mixer a gone owner's voice is never started.** The unchanged tree started it and the owner's absence ended it a frame later: a start and a stop for a sound that was never to sound, which on a real mixer is a blip. The check runs before
/// `start_pending`, so the output is not asked at all.
#[test]
fn a_gone_owners_voice_is_never_started_on_the_crates_mixer() {
    let mut app = app(engine(false), SirenSettings::default(), SirenSounds::new(), false);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    app.world_mut().resource_mut::<SirenSounds>().insert("engine", handle);
    let fake = Counting::default();
    app.insert_resource(Output(Box::new(fake.clone())));
    app.insert_resource(Backend(true));

    let live = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(Play::cue("engine").on(live));
    frames(&mut app, 2);
    assert_eq!(fake.started.load(Ordering::SeqCst), 1, "the control: a live owner's voice is started");

    let gone = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(gone);
    app.world_mut().write_message(Play::cue("engine").on(gone));
    frames(&mut app, 3);
    assert_eq!(
        fake.started.load(Ordering::SeqCst),
        1,
        "a gone owner's voice is not started: {:?}",
        lines(&app)
    );
    assert_eq!(said(&app, "`engine`: owner despawned"), 1, "and says why it ended: {:?}", lines(&app));
}

/// **A voice names its owner, and is not its child.** `VoiceOwner` is the one handle a game needs to ask whose a voice is; the hierarchy is not involved — no `ChildOf` on the voice, no `Children` on the owner — which is what removed the panic for an owner not
/// spawned yet, the double transform and the false line.
#[test]
fn a_voice_names_its_owner_and_is_not_its_child() {
    let mut app = world_app(false);
    let owner = app.world_mut().spawn(Transform::default()).id();
    frames(&mut app, 1);
    app.world_mut().write_message(Play::cue("engine").on(owner));
    frames(&mut app, 2);
    let voice = played(&mut app).remove(0).entity;

    assert_eq!(app.world().get::<VoiceOwner>(voice), Some(&VoiceOwner(owner)), "the voice says whose it is");
    assert!(app.world().get::<ChildOf>(voice).is_none(), "and is not a child");
    assert!(
        app.world().get::<Children>(owner).is_none(),
        "so the owner has no children to despawn, or to inherit from"
    );
}

/// **A sound that will never be heard takes nothing from one that is.** The cue is at its default `max_instances` of one, so a second play would steal the first — and a play on an owner that is already gone is despawned in its own frame. The unchanged tree
/// let it steal (`stolen by engine (cue)`) before it ended: a playing engine lost to a sound that never played.
#[test]
fn a_play_on_a_gone_owner_takes_nothing_from_a_sound_that_is_playing() {
    let mut app = world_app(false);
    let live = app.world_mut().spawn_empty().id();
    frames(&mut app, 1);
    app.world_mut().write_message(Play::cue("engine").on(live));
    frames(&mut app, 2);
    assert_eq!(voices(&mut app), 1, "the control: one engine is playing");

    let gone = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(gone);
    app.world_mut().write_message(Play::cue("engine").on(gone));
    frames(&mut app, 3);

    assert_eq!(voices(&mut app), 1, "the engine that was playing still is: {:?}", lines(&app));
    assert_eq!(said(&app, "stolen"), 0, "and nothing says it was stolen: {:?}", lines(&app));
    assert_eq!(
        said(&app, "`engine`: owner despawned"),
        1,
        "the doomed play ended for its reason: {:?}",
        lines(&app)
    );
}

/// **A placed sound on an owner spawned in the same frame follows it once it exists** — a projectile spawned with a whoosh. The owner is allocated and not spawned when the play is answered, so there is no `GlobalTransform` to measure against; the offset is
/// fixed the first frame it has one, from the voice's world position, so the sound is at `at` from its first frame and moves with the owner from then on.
#[test]
fn a_placed_sound_on_an_owner_spawned_in_the_same_frame_follows_it_once_it_exists() {
    let mut app = world_app(true);
    let owner = app.world().entity_allocator().alloc();
    app.world_mut().write_message(Play::cue("engine").on(owner).at(WORLD));
    app.update();
    let voice = played(&mut app).remove(0).entity;
    let at = |app: &App| {
        app.world().get::<GlobalTransform>(voice).expect("the voice's global transform").translation()
    };
    assert!(
        (at(&app) - WORLD).length() < 1e-3,
        "before its owner exists the sound is where it was put: {}",
        at(&app)
    );

    app.world_mut()
        .spawn_at(owner, Transform::from_translation(WORLD))
        .expect("the reserved id takes its entity");
    frames(&mut app, 3);
    assert!(
        (at(&app) - WORLD).length() < 1e-3,
        "the owner exists at the place, and the sound has not moved: {}",
        at(&app)
    );

    app.world_mut().entity_mut(owner).get_mut::<Transform>().expect("the owner's transform").translation =
        WORLD + Vec3::new(10.0, 0.0, 0.0);
    frames(&mut app, 3);
    assert!(
        (at(&app) - (WORLD + Vec3::new(10.0, 0.0, 0.0))).length() < 1e-3,
        "and then it follows: {}",
        at(&app)
    );
}
