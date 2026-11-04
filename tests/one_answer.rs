//! **A play is answered once, and the entity a `Played` names exists when the frame ends.**
//!
//! `Played::entity` is documented as *"the voice entity, for `StopVoice` or for a game that hangs its own data on it"* — a handle the crate hands out. On the crate's own mixer a voice was spawned and `Played` written in
//! `play_requests`, and its bytes were decoded in `start_pending`, the next system in the chain: when `rodio` refused them the entity was despawned **in the frame it was announced**, with a `Refused` as a second answer to the
//! one play. A game that hangs data on `Played::entity` — the use the doc names — then **panics in Bevy's command error handler** (*"Entity despawned"*), naming its own system and not the file
//! (`docs/loop/runs/2026-11-04-118/evidence/before/`). The README, the crate docs and `AGENTS.md` all say the two backends *answer identically*; on Bevy's path iteration 117 already refuses such bytes at the play.
//!
//! Gregory (*Game Engine Architecture*, §16.5.3) on handles, Hunt and Thomas (*The Pragmatic Programmer*, tip 37) on contracts, and Barr et al. (IEEE TSE 2015, §4.1) on invariants as oracles: **a `Played` names an entity that
//! exists** is the postcondition, and the second test checks it across the scenarios the crate can build rather than one.
//!
//! The mixer's path is built with a fake `AudioOutput` (a headless app has no device): `Output` and `Backend(true)` make the crate's own graph own the voice, and the fake says what a real backend would — that the bytes cannot be decoded.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::audio::AudioSource;
use bevy::prelude::*;
use bevy_siren::output::{AudioOutput, Backend, Output};
use bevy_siren::{
    Cue, CueTable, Entry, Play, Played, SirenLog, SirenMix, SirenSettings, SirenSounds, SirenSystems,
    SoundSlot,
};

use crate::helpers::{app, played, refused, voices};

/// A backend that starts every sample, or — when `refuse` — refuses them all, as `rodio` does for bytes it cannot decode, **and says so before a voice exists** (`decode_error`), which is the contract an output whose `play`
/// can refuse signs.
#[derive(Clone, Default)]
struct Counting {
    started: Arc<AtomicUsize>,
    /// How many times the crate asked this backend whether bytes can be decoded.
    asked: Arc<AtomicUsize>,
    refuse: bool,
}

impl AudioOutput for Counting {
    fn set_bus_db(&mut self, _: &str, _: f32) {}
    fn set_voice_db(&mut self, _: &str, _: usize, _: f32) {}
    fn stop_voice(&mut self, _: &str, _: usize) {}
    fn decode_error(&self, _: &Arc<[u8]>) -> Option<String> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        self.refuse.then(|| "the fake backend cannot decode these bytes".to_owned())
    }
    fn play(&mut self, _: &str, _: &Arc<[u8]>, _: f32, _: f32, _: bool) -> Option<usize> {
        if self.refuse { None } else { Some(self.started.fetch_add(1, Ordering::SeqCst)) }
    }
    fn finished(&self, _: &str, _: usize) -> bool {
        false
    }
}

/// An app whose cue `"click"` plays the sound `"s"` — placeholder bytes, **already loaded** — on the crate's own mixer (`Output` and `Backend(true)`), where the fake output starts or refuses what it is handed.
fn mixer_app(refuse: bool) -> (App, Counting) {
    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("s")], ..Cue::new("click") }]);
    let mut app = app(table, SirenSettings::default(), SirenSounds::new(), false);
    let handle = app
        .world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .add(AudioSource { bytes: Arc::from(vec![0u8; 4]) });
    app.world_mut().resource_mut::<SirenSounds>().insert("s", handle);
    let fake = Counting { refuse, ..Default::default() };
    app.insert_resource(Output(Box::new(fake.clone())));
    app.insert_resource(Backend(true));
    (app, fake)
}

#[derive(Component)]
struct Tag;

/// A game that hangs its own data on the voice — the use `Played::entity` documents — in a system after the crate's `Play` set, as a game reading the message in the frame it is written would.
fn tag_every_voice(mut played: MessageReader<Played>, mut commands: Commands) {
    for answer in played.read() {
        commands.entity(answer.entity).insert(Tag);
    }
}

/// **One play, one answer — and a game that tags the entity it was told about does not crash.** The unchanged tree answered `Played`, then `Refused` in the same frame, despawned the entity, and panicked the tagging
/// system. (`evidence/before/front-door-probe-on-the-unchanged-tree.txt`.)
#[test]
fn bytes_the_crates_mixer_refuses_are_refused_at_the_play_and_nothing_is_announced() {
    let (mut app, fake) = mixer_app(true);
    app.add_systems(Update, tag_every_voice.after(SirenSystems::Play));
    app.world_mut().write_message(Play::cue("click"));

    let frames = catch_unwind(AssertUnwindSafe(|| {
        for _ in 0..3 {
            app.update();
        }
    }));
    assert!(
        frames.is_ok(),
        "a game system that hangs data on `Played::entity` ran for three frames without a panic"
    );

    assert!(played(&mut app).is_empty(), "nothing plays on a refusal, so nothing is announced as played");
    let answers = refused(&mut app);
    assert_eq!(answers.len(), 1, "one play, one answer: {answers:?}");
    assert_eq!(voices(&mut app), 0, "no voice was spawned for bytes the backend cannot decode");
    assert_eq!(
        app.world().resource::<SirenMix>().voices_on("Effects"),
        0,
        "and nothing is counted against the budget"
    );
    assert_eq!(fake.started.load(Ordering::SeqCst), 0, "and the backend was never asked to start them");

    let log: Vec<String> =
        app.world().resource::<bevy_siren::SirenLog>().iter().map(|entry| entry.line()).collect();
    assert!(!log.iter().any(|l| l.contains(" played ")), "the log never says it played: {log:?}");
    assert!(
        log.iter().any(|l| l.contains("refused") && l.contains("cannot be decoded") && l.contains("`s`")),
        "and names the sound and says the bytes cannot be decoded: {log:?}"
    );
}

/// **One answer per play, and a voice that is gone is explained** — across the scenarios the crate can build. After every frame: each play written before it was answered by exactly one of `Played` or `Refused` (a `Played`
/// and then a `Refused` for one play is the defect the first test pins); and every entity a `Played` named that is **gone** when the frame ends was stopped for a reason the log states. A voice stolen by a later request in the frame it was announced,
/// or stopped by a `StopCue` in it, is despawned in that frame — it never sounded — and the log says `stopped … stolen by …` rather than nothing; that is the one way a `Played` entity is gone at the end of its frame, and
/// `Played::entity`'s doc says so.
#[test]
fn every_play_is_answered_once_and_a_voice_that_is_gone_is_explained() {
    /// Runs `frames` frames, the first with `plays` requests already written, and checks both rules after each.
    fn check(app: &mut App, plays: usize, frames: usize, what: &str) {
        for frame in 0..frames {
            app.update();
            let announced = played(app);
            let refusals = refused(app);
            let expected = if frame == 0 { plays } else { 0 };
            assert_eq!(
                announced.len() + refusals.len(),
                expected,
                "{what}: frame {frame}: one answer per play, got {} played and {} refused",
                announced.len(),
                refusals.len()
            );
            for answer in announced {
                if app.world().get_entity(answer.entity).is_err() {
                    let explained = app
                        .world()
                        .resource::<SirenLog>()
                        .iter()
                        .any(|entry| matches!(entry, Entry::Stopped { cue, .. } if *cue == answer.cue));
                    assert!(
                        explained,
                        "{what}: frame {frame}: `Played` named {:?}, which is gone when the frame ends, and the log has no stop for `{}`",
                        answer.entity, answer.cue
                    );
                }
            }
        }
    }

    // Bevy's path, a plain play.
    let table = CueTable::new(vec![Cue { sounds: vec![SoundSlot::new("s")], ..Cue::new("click") }]);
    let mut bevy = app(table, SirenSettings::default(), crate::helpers::sounds(&["s"]), false);
    bevy.world_mut().remove_resource::<Output>();
    bevy.world_mut().write_message(Play::cue("click"));
    check(&mut bevy, 1, 3, "Bevy's path");

    // The crate's mixer, bytes the backend starts.
    let (mut mixer, fake) = mixer_app(false);
    mixer.world_mut().write_message(Play::cue("click"));
    check(&mut mixer, 1, 3, "the crate's mixer");
    assert_eq!(fake.started.load(Ordering::SeqCst), 1, "the backend started it");

    // A play that steals the voice the same cue already holds (`max_instances` is 1 by default), with a stop in the same frame.
    let (mut stealing, _) = mixer_app(false);
    stealing.world_mut().write_message(Play::cue("click"));
    check(&mut stealing, 1, 1, "the first play");
    stealing.world_mut().write_message(Play::cue("click"));
    stealing.world_mut().write_message(bevy_siren::StopCue { cue: "click".to_owned() });
    check(&mut stealing, 1, 3, "a steal and a stop in one frame");

    // **Two plays of one cue in one frame**: `max_instances` is 1, so the second steals the first in the frame the first was announced.
    let (mut same_frame, _) = mixer_app(false);
    same_frame.world_mut().write_message(Play::cue("click"));
    same_frame.world_mut().write_message(Play::cue("click"));
    check(&mut same_frame, 2, 3, "a play stolen in the frame it was announced");

    // **A play stopped in the frame it was announced.**
    let (mut stopped, _) = mixer_app(false);
    stopped.world_mut().write_message(Play::cue("click"));
    stopped.world_mut().write_message(bevy_siren::StopCue { cue: "click".to_owned() });
    check(&mut stopped, 1, 3, "a play stopped in the frame it was announced");

    // The refused one: bytes the backend cannot decode are answered once, by a refusal — nothing is announced, and no voice is left behind.
    let (mut refusing, _) = mixer_app(true);
    refusing.world_mut().write_message(Play::cue("click"));
    check(&mut refusing, 1, 3, "bytes the crate's mixer refuses");
    assert_eq!(voices(&mut refusing), 0, "and no voice is left behind");
}

/// **A sound is asked of the mixer once, however often it is played.** `decode_error` is a decoder construction (0.6 – 0.8 ms for a Vorbis) and the crate asks it behind the same per-buffer memory as Bevy's decoder: five plays,
/// one question; an edit to the file is a new buffer, and is asked again — once.
#[test]
fn a_sound_is_asked_of_the_mixer_once_however_often_it_is_played() {
    let (mut app, fake) = mixer_app(false);
    app.world_mut().resource_mut::<CueTable>().insert(Cue {
        sounds: vec![SoundSlot::new("s")],
        max_instances: 0,
        ..Cue::new("click")
    });
    for _ in 0..5 {
        app.world_mut().write_message(Play::cue("click"));
        app.update();
    }
    assert_eq!(fake.started.load(Ordering::SeqCst), 5, "five plays were started");
    assert_eq!(fake.asked.load(Ordering::SeqCst), 1, "and the backend was asked about the bytes once");

    let handle = app.world().resource::<SirenSounds>().get("s").expect("the sound has a handle").clone();
    app.world_mut()
        .resource_mut::<Assets<AudioSource>>()
        .insert(handle.id(), AudioSource { bytes: Arc::from(vec![1u8; 4]) })
        .expect("the asset is replaced");
    for _ in 0..3 {
        app.world_mut().write_message(Play::cue("click"));
        app.update();
    }
    assert_eq!(
        fake.asked.load(Ordering::SeqCst),
        2,
        "an edited file is a new buffer, asked again — once, not three times"
    );
}

/// **A verdict is the asker's own.** Bytes the crate's mixer accepts and Bevy's decoder refuses — a fake backend that accepts anything, and four zero bytes — are played on the mixer's path; the same bytes in the same world with Bevy
/// owning playback are refused, not answered from the other backend's memory (`rodio` builds Bevy's decoder and the mixer's differently, and a verdict borrowed across them would start a voice that Bevy then panics on).
#[test]
fn a_verdict_is_the_askers_own() {
    let (mut app, _) = mixer_app(false);
    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(played(&mut app).len(), 1, "the mixer accepts the bytes, so the play is announced");

    app.insert_resource(Backend(false));
    app.world_mut().write_message(Play::cue("click"));
    app.update();
    assert_eq!(
        played(&mut app).len(),
        0,
        "with Bevy owning playback the same bytes are Bevy's to decode, and it refuses them"
    );
    let answers = refused(&mut app);
    assert_eq!(answers.len(), 1, "so the play is refused at the play: {answers:?}");
}
