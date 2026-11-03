//! **One request, one answer.**
//!
//! Gameplay writes a [`Play`] and the crate answers with exactly one of [`Played`] or [`Refused`]. A
//! refusal is never silent: the reason names the missing cue, the missing file, the bus or the budget,
//! because a sound that does not happen with no explanation is the hardest bug in a game to find.
//!
//! **Both backends answer identically.** The entity a play spawns carries [`Voice`] either way and the
//! same [`Played`] is written, so a test, the editor's panel and a game's own meter see the decision
//! whether the samples go through Bevy's `AudioPlayer` or through the crate's own mixer.
//!
//! # The order of the questions
//!
//! Is there such a cue → does it have a sample → is there a file for that sample → is there a bus → is
//! there a voice free → is there an ear. That is the order a person asks them, and the first one that
//! fails is the answer. Nothing plays when any of them fails: a refusal is not a quiet success.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::cue::{Cue, CueTable, Mode};
use crate::log::{Entry, SirenLog};
use crate::mix::{DuckClock, SirenSettings, Stopping, UNSTEALABLE_PRIORITY};
use crate::model::{
    Candidate, Rng, SILENT_DB, VariantPicker, backend_spatial_gain, chain_db, distance_gain, finite_or,
    gain_to_db, semitones_to_ratio, steal,
};
use crate::sounds::SirenSounds;

/// Ask for a cue to play.
///
/// Build it with [`Play::cue`] and refine it: [`on`](Play::on) attaches the play to an entity (a footstep
/// belongs to the foot), [`at`](Play::at) places it in the world, [`gain`](Play::gain) and
/// [`pitch`](Play::pitch) adjust it for this play only.
///
/// ```no_run
/// # use bevy::prelude::*;
/// # use bevy_siren::*;
/// # fn ask(mut play: MessageWriter<Play>, foot: Entity) {
/// play.write(Play::cue("footstep.grass"));
/// play.write(Play::cue("footstep.grass").on(foot).at(Vec3::new(2.0, 0.0, 1.0)));
/// play.write(Play::cue("ui.cancel").gain(-3.0));
/// # }
/// ```
#[derive(Message, Clone, Debug, PartialEq)]
pub struct Play {
    /// The cue to play, by name.
    pub cue: String,
    /// The entity the sound belongs to: a footstep's foot, an engine's vehicle.
    pub target: Option<Entity>,
    /// Where in the world the sound is, for a spatial cue.
    pub position: Option<Vec3>,
    /// A gain for this play only, added to the cue's.
    pub gain_db: Option<f32>,
    /// A detune for this play only, added to the cue's own jitter.
    pub pitch: Option<f32>,
}

impl Play {
    /// A play of the cue called `name`.
    pub fn cue(name: impl Into<String>) -> Self {
        Self { cue: name.into(), target: None, position: None, gain_db: None, pitch: None }
    }

    /// The sound belongs to `entity`.
    pub fn on(mut self, entity: Entity) -> Self {
        self.target = Some(entity);
        self
    }

    /// The sound is at `position` in the world.
    pub fn at(mut self, position: Vec3) -> Self {
        self.position = Some(position);
        self
    }

    /// Adjusts this play by `db`.
    pub fn gain(mut self, db: f32) -> Self {
        self.gain_db = Some(db);
        self
    }

    /// Detunes this play by `semitones`.
    pub fn pitch(mut self, semitones: f32) -> Self {
        self.pitch = Some(semitones);
        self
    }
}

/// A play that started: which variant, on which bus, at what gain and pitch.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct Played {
    /// The voice entity, for [`StopVoice`] or for a game that hangs its own data on it.
    pub entity: Entity,
    /// The cue that was asked for.
    pub cue: String,
    /// The variant that was chosen.
    pub sound: String,
    /// The bus it plays on.
    pub bus: String,
    /// The gain it started at, before the bus's own level and the master.
    pub gain_db: f32,
    /// The detune it started at, in semitones.
    pub pitch: f32,
}

/// A play that did not happen, and why.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct Refused {
    /// The cue that was asked for.
    pub cue: String,
    /// The reason, as a value a game can match on.
    pub reason: Refusal,
}

/// Why a [`Play`] was refused. Every variant names what is missing, so a refusal is actionable.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// No cue of that name is in the table.
    UnknownCue,
    /// The cue has no variants, so there is nothing to play.
    NoSounds,
    /// The cue's chosen variant names a `sound` key with no handle in [`SirenSounds`], or one whose bytes the
    /// decoder refused.
    ///
    /// Two causes, one sentence, because both mean the same thing to a project: **this sound cannot be played, and
    /// here is the key to go and look at**. The missing-handle case is found when the play is answered; the
    /// undecodable case when the bytes first load, which is why this variant is produced from two places.
    MissingSound {
        /// The key a project maps to a file — never the cue's name.
        sound: String,
    },
    /// The cue names a bus the mix table does not have.
    UnknownBus {
        /// The bus it names.
        bus: String,
    },
    /// The voice budget could not be made room in — the cue's own cap, its bus's, or the table's.
    VoiceLimit {
        /// The bus that had no room, when one did.
        bus: Option<String>,
    },
    /// The play asked for a world position while no `SpatialListener` exists, so there is no ear to place
    /// it against.
    SpatiallyMute(Vec3),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCue => write!(f, "no cue of that name"),
            Self::NoSounds => write!(f, "the cue has no sounds"),
            Self::MissingSound { sound } => {
                write!(f, "no handle or undecodable bytes for sound `{sound}`")
            }
            Self::UnknownBus { bus } => write!(f, "bus `{bus}` is not in the mix table"),
            Self::VoiceLimit { bus: Some(bus) } => write!(f, "no voice free on bus `{bus}`"),
            Self::VoiceLimit { bus: None } => write!(f, "no voice free"),
            Self::SpatiallyMute(p) => write!(f, "no SpatialListener to place a sound at {p}"),
        }
    }
}

/// A live sound. The entity also carries Bevy's `AudioPlayer` and, once it starts, `AudioSink` — unless the
/// crate's own mixer owns playback, where [`MixerVoice`] names the voice inside it instead.
///
/// **The cue's identity is here, not in the audio crate.** A test with no audio device still sees which
/// cue played, which variant, on which bus, at what gain — everything a decision consists of.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Voice {
    /// The cue that was played.
    pub cue: String,
    /// The bus it plays on.
    pub bus: String,
    /// The cue's priority: higher survives a full budget. At or above
    /// [`UNSTEALABLE_PRIORITY`] it is never taken.
    pub priority: u8,
    /// When it started, on real time.
    pub start_s: f64,
    /// Its own gain in dB, before the bus and the master, spatial falloff included.
    pub base_db: f32,
    /// The variant it chose, for a log that says which recording a player heard.
    pub variant: usize,
    /// The detune it started at, in semitones.
    pub semitones: f32,
    /// Whether it ends on its own or loops until stopped.
    pub mode: Mode,
    /// Whether the voice budget may take it back: music, stingers and dialogue are not stolen.
    pub unstealable: bool,
}

/// What the audio backend adds to a spatial voice's own gain: the panner's distance law, divided out so a
/// cue's [`Attenuation`](crate::cue::Attenuation) is the one a project writes and the backend's stereo
/// placement is kept.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Compensation {
    /// The dB to add to the voice's own gain.
    pub db: f32,
}

/// The sample a voice plays, so whichever backend owns playback can start it — Bevy through the `AudioPlayer`
/// spawned beside it, the crate's mixer through [`start_pending`](crate::playback::start_pending) once the bytes
/// are loaded.
#[derive(Component, Clone, Debug)]
pub struct Samples(pub Handle<AudioSource>);

/// The index of a voice inside the crate's own mixer's bus track. Inserted only when that mixer owns
/// playback; the type is always here so a query can name it either way.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MixerVoice(pub usize);

/// The mixer has reported this voice's sample ended.
///
/// With Bevy's backend a once-voice's end is `AudioSink::empty`; with the crate's own there is no sink, so the
/// mixer's player empty is recorded here. [`Voice`] carries no such flag: the sample's end is not one of the
/// crate's decisions.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Completes {
    done: bool,
}

impl Completes {
    /// True when the sample has ended.
    pub fn finished(&self) -> bool {
        self.done
    }
}

/// Ask for a live voice to stop, fading over [`DEFAULT_FADE_S`](crate::model::DEFAULT_FADE_S).
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StopVoice {
    /// The voice entity, as [`Played::entity`] reported it.
    pub entity: Entity,
}

/// Ask for every voice of a cue to stop.
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct StopCue {
    /// The cue's name.
    pub cue: String,
}

/// The crate's one random stream and its per-cue variant history.
///
/// A [`Resource`] rather than a `Local<..>` because two systems draw from it (a play's variant and its
/// detune) and a seeded stream must not restart when a system's local does.
///
/// **Its seed is the mix table's.** [`SirenSettings::seed`](crate::mix::SirenSettings::seed) is what a project writes,
/// and [`seed_choices`] replaces the stream when that number changes — so `seed: 7` and `seed: 99` play different
/// sequences and the same number plays the same one twice.
#[derive(Resource, Clone, Debug)]
pub struct SirenChoices {
    /// The stream every draw comes from.
    pub rng: Rng,
    /// The seed `rng` was built from, so [`seed_choices`] can notice the mix table moving.
    seeded: u64,
    pickers: HashMap<String, VariantPicker>,
}

impl SirenChoices {
    /// A stream seeded from `seed`.
    pub fn new(seed: u64) -> Self {
        let rng = Rng::new(seed);
        // **The seed the stream was built from**, not the one asked for: `Rng::new(0)` maps to
        // `Rng::ZERO_SEED`, so recording the argument made `seed()` report a number the stream did not come from —
        // and `seed_choices` compare against that same number to decide whether to re-seed.
        Self { seeded: rng.seed(), rng, pickers: HashMap::new() }
    }

    /// The seed this stream was built from.
    ///
    /// This is the **effective** seed: a project that writes `seed: 0` is told
    /// [`Rng::ZERO_SEED`](crate::model::Rng::ZERO_SEED), which is the value its stream actually started at, and the same
    /// value [`SirenSettings::default_seed`](crate::mix::SirenSettings::default_seed) returns. A nonzero seed is reported
    /// as written.
    pub fn seed(&self) -> u64 {
        self.seeded
    }

    /// The variant index to play for `cue`.
    ///
    /// **By key, not by index.** A cue may list one file twice — a copy-pasted line, a generated table — and those are
    /// two indices and one sound. `pick` refused indices, so under the default `AvoidRepeat { avoid: 1 }` such a cue
    /// played the same recording **eight times running**: the machine-gun footstep `Repeat` exists to kill.
    fn pick(&mut self, cue: &Cue) -> Option<usize> {
        let avoid = cue.repeat.avoid();
        let capacity = usize::from(avoid).max(1);
        let keys: Vec<&str> = cue.sounds.iter().map(|slot| slot.sound.as_str()).collect();
        let picker = self.pickers.entry(cue.name.clone()).or_insert_with(|| VariantPicker::new(capacity));
        picker.pick_from(&keys, avoid, &mut self.rng)
    }

    /// Forgets every variant history, so the next play of each cue is unconstrained.
    pub fn reset_histories(&mut self) {
        for picker in self.pickers.values_mut() {
            picker.reset();
        }
    }
}

impl Default for SirenChoices {
    fn default() -> Self {
        Self::new(SirenSettings::default_seed())
    }
}

/// **Makes [`SirenSettings::seed`](crate::mix::SirenSettings::seed) the stream the crate draws from.**
///
/// The mix table is the project's file; a crate constant is not a seed a project can choose. This runs first in
/// [`SirenSystems::Choose`](crate::SirenSystems::Choose), before the music starts a layer, so the first play after a
/// change already draws from the new stream.
///
/// A changed seed **replaces** the stream and every variant history rather than continuing from the old position: a
/// different seed means a different sequence from its first draw, and a history carried across would make the first
/// play of the new stream depend on the old one. Nothing happens on a frame whose seed did not move, and the seed is
/// configuration rather than a decision, so nothing is logged — the log is for what happened to a sound.
pub fn seed_choices(settings: Res<SirenSettings>, mut choices: ResMut<SirenChoices>) {
    if choices.seeded == settings.seed {
        return;
    }
    *choices = SirenChoices::new(settings.seed);
}

/// One voice as the budget sees it: what `room_for` needs, owned so a request this frame can add its own voice
/// without borrowing a query it cannot see.
#[derive(Clone, Debug, PartialEq)]
struct Live {
    entity: Entity,
    cue: String,
    bus: String,
    priority: u8,
    start_s: f64,
    unstealable: bool,
}

impl Live {
    /// **A music layer's voice is unstealable whatever priority its cue authored.** `UNSTEALABLE_PRIORITY` is a
    /// *label* a cue carries; a layer is a **stem**, and R4's "music, stingers and dialogue are not stolen" is about
    /// the sound, not about whether the score's author remembered `priority: 180`. The plugin marks every voice it
    /// adopts with [`MusicVoice`](crate::music::MusicVoice), which is the fact this row reads.
    fn of((entity, voice, music): (Entity, &Voice, Option<&crate::music::MusicVoice>)) -> Self {
        Self {
            entity,
            cue: voice.cue.clone(),
            bus: voice.bus.clone(),
            priority: voice.priority,
            start_s: voice.start_s,
            unstealable: voice.unstealable || music.is_some(),
        }
    }

    fn candidate(&self, index: usize) -> Candidate {
        Candidate { index, priority: self.priority, start_s: self.start_s, unstealable: self.unstealable }
    }
}

/// What a request resolved to, before it becomes an entity.
struct Started {
    variant: usize,
    sound: String,
    bus: String,
    gain_db: f32,
    pitch: f32,
    handle: Handle<AudioSource>,
    compensation_db: f32,
    /// Whether this voice is placed in the world: the cue is spatial **and** the play has a position. One predicate,
    /// decided where the attenuation is computed, so the panner and the `Transform` cannot disagree with it.
    spatial: bool,
    priority: u8,
    mode: Mode,
    /// The voices this play takes over to fit the budget, and which cap chose each.
    stolen: Vec<(Entity, Cap)>,
}

/// **Turns [`Play`] into an entity or a [`Refused`].** Runs in [`SirenSystems::Play`](crate::SirenSystems).
#[allow(clippy::too_many_arguments)]
pub fn play_requests(
    mut requests: MessageReader<Play>,
    table: Res<CueTable>,
    sounds: Res<SirenSounds>,
    settings: Res<SirenSettings>,
    mut choices: ResMut<SirenChoices>,
    mut clock: ResMut<DuckClock>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
    listener: Query<(&GlobalTransform, &SpatialListener)>,
    voices: Query<(Entity, &Voice, Option<&crate::music::MusicVoice>), Without<Stopping>>,
    backend: Res<crate::output::Backend>,
    output: Option<Res<crate::output::Output>>,
    mut played: MessageWriter<Played>,
    mut refused: MessageWriter<Refused>,
    mut commands: Commands,
) {
    let now = real.elapsed_secs_f64();
    // **The live list shrinks as this frame's own requests fill the budget.** A snapshot taken before the loop
    // would let three requests in one frame each see an empty bus and all three play, which breaks a cap the
    // crate promises to keep; a voice this frame marked stopping no longer counts.
    let mut live: Vec<Live> = voices.iter().map(Live::of).collect();
    for request in requests.read() {
        match answer(request, table.as_ref(), &sounds, &settings, &mut choices, &listener, &live, now) {
            Ok(started) => {
                for (victim, cap) in &started.stolen {
                    // **Which cap took the voice, not only who asked.** A `footstep.grass` request can take a voice
                    // because the *cue* is at `max_instances` or because its *bus* is at `max_voices`, and the two want
                    // different edits from an author — raise the cue's x2, or the bus's budget. The old sentence said
                    // `stolen by footstep.grass` for both, so a step that took three voices logged two identical
                    // lines and neither said which number to change.
                    let why = format!("stolen by {} ({})", request.cue, cap.as_str());
                    commands.entity(*victim).insert(Stopping { since_s: now, why: why.clone() });
                    if let Some(voice) = live.iter().find(|v| v.entity == *victim) {
                        log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why });
                    }
                    live.retain(|v| v.entity != *victim);
                }
                // **A voice is mixer-owned only when a mixer is there to own it.** `Backend` is a public resource
                // whose flag a host may set, and `Backend(true)` with no `Output` used to give a voice **no
                // `AudioPlayer` and no mixer** - it could not sound, could not end, and could not be retired, so it
                // held its budget slot for the life of the process while `Played` said it had played.
                // `start_pending`'s doc already promises the fallback ("without an `Output` ... Bevy's `AudioPlayer`
                // owns every voice"); this is what makes that promise true rather than a claim about a resource the
                // spawn never read.
                let mixer_owns = backend.0 && output.is_some();
                let entity = spawn_voice(&mut commands, request, &started, now, mixer_owns);
                // This frame's own voice counts against the next request in the list.
                live.push(Live {
                    entity,
                    cue: request.cue.clone(),
                    bus: started.bus.clone(),
                    priority: started.priority,
                    start_s: now,
                    // A play written this frame has not been adopted by the music plugin yet, so it cannot be a layer.
                    unstealable: started.priority >= UNSTEALABLE_PRIORITY,
                });
                if settings.ducks.iter().any(|d| d.source == started.bus) {
                    clock.fire_all(&settings, &started.bus, now);
                    for duck in settings.ducks.iter().filter(|d| d.source == started.bus) {
                        log.push(Entry::Ducked {
                            at_s: now,
                            bus: duck.target.clone(),
                            by_db: duck.by_db,
                            by: request.cue.clone(),
                        });
                    }
                }
                log.push(Entry::Played {
                    at_s: now,
                    cue: request.cue.clone(),
                    sound: started.sound.clone(),
                    bus: started.bus.clone(),
                    gain_db: started.gain_db,
                });
                played.write(Played {
                    entity,
                    cue: request.cue.clone(),
                    sound: started.sound.clone(),
                    bus: started.bus.clone(),
                    gain_db: started.gain_db,
                    pitch: started.pitch,
                });
            }
            Err(reason) => {
                log.push(Entry::Refused { at_s: now, cue: request.cue.clone(), reason: reason.to_string() });
                refused.write(Refused { cue: request.cue.clone(), reason });
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn answer(
    request: &Play,
    table: &CueTable,
    sounds: &SirenSounds,
    settings: &SirenSettings,
    choices: &mut SirenChoices,
    listener: &Query<(&GlobalTransform, &SpatialListener)>,
    live: &[Live],
    now: f64,
) -> Result<Started, Refusal> {
    let Some(cue) = table.get(&request.cue) else {
        return Err(Refusal::UnknownCue);
    };
    if !cue.has_sounds() {
        return Err(Refusal::NoSounds);
    }
    let Some(variant) = choices.pick(cue) else {
        return Err(Refusal::NoSounds);
    };
    let Some(slot) = cue.sounds.get(variant) else {
        return Err(Refusal::NoSounds);
    };
    let Some(handle) = sounds.get(&slot.sound) else {
        return Err(Refusal::MissingSound { sound: slot.sound.clone() });
    };
    let bus = cue.bus_name().to_owned();
    if !settings.bus_names().contains(&bus.as_str()) {
        return Err(Refusal::UnknownBus { bus });
    }

    // The budget comes before the ear: a request that cannot play has no place to play at.
    //
    // **The bus is named only when the bus is what refused.** `VoiceLimit::bus` is documented as "the bus that had no
    // room, **when one did**", and a full table or a cue at its own `max_instances` leaves every bus with room: naming
    // one sends a person to a bus row they will find empty. Gregory §14.5.8.4 has a designer set a group's limit and
    // the engine's limit separately; a refusal has to keep them apart the same way.
    let stolen = match room_for(live, cue, settings) {
        Ok(stolen) => stolen,
        Err(Cap::Bus) => return Err(Refusal::VoiceLimit { bus: Some(bus) }),
        Err(Cap::Cue | Cap::Table) => return Err(Refusal::VoiceLimit { bus: None }),
    };

    let mut pitch = cue.pitch.at(choices.rng.next_f32());
    if let Some(extra) = request.pitch {
        pitch += extra;
    }
    // **A detune that is not a number is no detune, and `Played` says so.** `semitones_to_ratio` already played a
    // non-finite pitch at `1.0x`, but `Started.pitch` carried the raw `NaN` into `Played`: the meter and the device
    // disagreed, which is the one thing R6's second half forbids. One rule with `distance_gain`: a non-finite caller
    // value is the neutral one for what it measures.
    // And the number `Played` reports is the **rate the device plays**, so a detune past the clamp says so.
    pitch = crate::model::played_semitones(finite_or(pitch, 0.0));
    let mut parts = vec![
        finite_or(cue.gain_db, 0.0),
        finite_or(slot.gain_db, 0.0),
        finite_or(request.gain_db.unwrap_or(0.0), 0.0),
    ];
    let mut compensation_db = 0.0;
    // **A voice is placed only if the cue is spatial and the play has a position.** This one predicate decides the
    // attenuation here, the panner in `spawn_voice`, and the `Transform` there too. A non-spatial cue given a position
    // used to be routed through the two-ear player by a *different* predicate (`position.is_some()`), which applied
    // rodio's `min(1, 1/d²)` — −38.7 dB at 8 m — while `Played.gain_db` reported 0.00: the sample and the meter said
    // different things, and the meter is what a game's own level readout believes.
    let spatial = cue.spatial && request.position.is_some();
    if spatial && let Some(at) = request.position {
        let Some((ear, ears)) = listener.iter().next() else {
            return Err(Refusal::SpatiallyMute(at));
        };
        let d = ear.translation().distance(at);
        parts.push(gain_to_db(distance_gain(cue.attenuation, d)));
        let backend = backend_spatial_gain(
            at.into(),
            ear.transform_point(ears.left_ear_offset).into(),
            ear.transform_point(ears.right_ear_offset).into(),
        );
        compensation_db = -gain_to_db(backend);
    }
    let gain_db = chain_db(&parts);
    let _ = now;
    Ok(Started {
        variant,
        sound: slot.sound.clone(),
        bus,
        gain_db,
        pitch,
        handle: handle.clone(),
        compensation_db,
        spatial,
        priority: cue.priority,
        mode: cue.mode,
        stolen,
    })
}

/// Which of the three hard caps could not be made room in.
///
/// Kept apart because they are three different things a project tunes, and a person reading a refusal needs to know
/// which one to go and edit: a cue's own polyphony, its bus's voices, or the table's whole budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cap {
    /// The cue's own `max_instances`.
    Cue,
    /// The bus's own `max_voices`.
    Bus,
    /// The table's `max_voices`.
    Table,
}

impl Cap {
    /// The word the log uses, so `stolen by footstep.grass (cue)` and `(bus)` say which number to raise.
    fn as_str(self) -> &'static str {
        match self {
            Self::Cue => "cue",
            Self::Bus => "bus",
            Self::Table => "table",
        }
    }
}

/// The voices to stop so one more fits, or the cap that could not be made room in.
///
/// Three caps, in the order a project thinks of them: the cue's own polyphony, its bus's voices, and the
/// table's whole budget. Each is hard: a voice that cannot be freed is a refusal, never a 33rd voice.
fn room_for(live: &[Live], cue: &Cue, settings: &SirenSettings) -> Result<Vec<(Entity, Cap)>, Cap> {
    let bus = cue.bus_name();
    let total = usize::from(settings.max_voices);
    let bus_limit = settings.bus_voice_limit(bus);
    let mut stop: Vec<(Entity, Cap)> = Vec::new();

    // The cue's own polyphony: its oldest voices go.
    if cue.max_instances > 0 {
        let same: Vec<&Live> = live.iter().filter(|v| v.cue == cue.name).collect();
        let excess = (same.len() + 1).saturating_sub(usize::from(cue.max_instances));
        for voice in same.iter().take(excess) {
            if voice.unstealable {
                return Err(Cap::Cue);
            }
            stop.push((voice.entity, Cap::Cue));
        }
    }

    // The bus's cap: its lowest-priority, then oldest, voices.
    let on_bus: Vec<usize> = live
        .iter()
        .enumerate()
        .filter(|(_, v)| v.bus == bus && !stop.iter().any(|(e, _)| *e == v.entity))
        .map(|(i, _)| i)
        .collect();
    let after = on_bus.len();
    if after >= bus_limit {
        let need = after + 1 - bus_limit;
        let chosen = steal(&candidates_of(live, &on_bus), need, bus_limit);
        if chosen.len() < need {
            return Err(Cap::Bus);
        }
        stop.extend(chosen.into_iter().map(|index| (live[index].entity, Cap::Bus)));
    }

    // The table's budget, across every bus.
    let after = live.len().saturating_sub(stop.len());
    if after >= total {
        let need = after + 1 - total;
        let all: Vec<usize> = live
            .iter()
            .enumerate()
            .filter(|(_, v)| !stop.iter().any(|(e, _)| *e == v.entity))
            .map(|(i, _)| i)
            .collect();
        let chosen = steal(&candidates_of(live, &all), need, total);
        if chosen.len() < need {
            return Err(Cap::Table);
        }
        stop.extend(chosen.into_iter().map(|index| (live[index].entity, Cap::Table)));
    }
    Ok(stop)
}

fn candidates_of(live: &[Live], indices: &[usize]) -> Vec<Candidate> {
    indices.iter().map(|index| live[*index].candidate(*index)).collect()
}

/// Spawns the voice entity and starts its sound.
///
/// With Bevy's backend the entity carries `AudioPlayer` + `PlaybackSettings`, and Bevy starts it — including the
/// spatial two-ear path when the play had a position. With the crate's mixer, the entity carries [`Samples`] and
/// no `AudioPlayer`: [`start_pending`] starts the sample on the mixer's own bus
/// and inserts [`MixerVoice`], so the two backends can never both play one request.
fn spawn_voice(
    commands: &mut Commands,
    request: &Play,
    started: &Started,
    now: f64,
    mixer_owns: bool,
) -> Entity {
    let voice = Voice {
        cue: request.cue.clone(),
        bus: started.bus.clone(),
        priority: started.priority,
        start_s: now,
        base_db: started.gain_db,
        variant: started.variant,
        semitones: started.pitch,
        mode: started.mode,
        unstealable: started.priority >= UNSTEALABLE_PRIORITY,
    };
    let mut spawned = commands.spawn((
        voice,
        Compensation { db: started.compensation_db },
        crate::mix::Applied { db: SILENT_DB },
        Samples(started.handle.clone()),
    ));
    // **Bevy's path is the one every build has**: the entity carries `AudioPlayer` and Bevy starts it, waiting
    // for the asset exactly as the crate's own playback does. The one exception is a build whose mixer owns
    // playback, where an `AudioPlayer` would make both backends play the same request.
    if !mixer_owns {
        use bevy::audio::{AudioPlayer, PlaybackMode, PlaybackSettings};
        let settings = PlaybackSettings {
            mode: if started.mode.is_loop() { PlaybackMode::Loop } else { PlaybackMode::Once },
            volume: bevy::audio::Volume::Linear(crate::model::db_to_gain(chain_db(&[
                started.gain_db,
                started.compensation_db,
            ]))),
            speed: semitones_to_ratio(started.pitch),
            spatial: started.spatial,
            ..PlaybackSettings::ONCE
        };
        spawned.insert((AudioPlayer::new(started.handle.clone()), settings));
    }
    // The `Transform` is what the panner reads, so it exists on exactly the voices that are placed.
    if started.spatial
        && let Some(at) = request.position
    {
        spawned.insert(Transform::from_translation(at));
    }
    if let Some(target) = request.target {
        spawned.insert(ChildOf(target));
    }
    spawned.id()
}
