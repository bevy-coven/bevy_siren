//! **Layered music that changes on the bar.**
//!
//! Adaptive music is vertical layering, not new tracks: the same performance with each instrument's gain
//! keyed to game state, so calm exploration and the combat that interrupts it are the same tempo, the same
//! beat, the same musicians (Khan et al. 2023, *Adaptive Background Music for a Fighting Game*,
//! 10.48550/arXiv.2303.15734). That is why a layer at [`SILENT_DB`] stays in the
//! mix: it costs one silent voice and keeps every layer phase-aligned, which is what makes an entry sound
//! like the same band getting louder rather than a second song starting.
//!
//! A change lands on a musical boundary — the next beat or the next bar — because a transition in the middle
//! of a phrase is heard as a mistake. [`model::next_boundary`](crate::model::next_boundary) computes it from
//! tempo alone, so a project does not ship a beat map with every track.
//!
//! A **stinger** is the exception that proves the rule: one hit, on the music bus, ducking what it plays over.
//! It arrives *now*, because a stinger that waited for the bar would miss the event it punctuates.
//!
//! # One play path
//!
//! The plugin never spawns a voice itself. It writes [`Play`] for each layer, and adopts the entity that
//! [`Played`] reports — so a music layer is a voice like any other: gated, budgeted, logged,
//! and mixed by the one mix system. A second spawn path would be a second set of rules.

use bevy::prelude::*;

use crate::cue::CueTable;
use crate::log::{Entry, SirenLog};
use crate::mix::{DuckClock, Stopping};
use crate::model::{DEFAULT_FADE_S, SILENT_DB, approach, next_boundary, tau_for_fade};
use crate::play::{Play, Played, Refusal, Voice};

/// A music score: the layers a track is made of, and its tempo.
///
/// With the `serde` feature this is a file a project ships and the Cauldron edits, beside the cue table.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MusicScore {
    /// The layers, in the order a project lists them.
    #[cfg_attr(feature = "serde", serde(default))]
    pub layers: Vec<MusicLayer>,
    /// The score's tempo. `0` is no beat grid: every transition is immediate, for a score that is a pad
    /// rather than a song.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tempo_bpm: f32,
    /// How many beats a bar has. Four is the default, and three is a waltz.
    #[cfg_attr(feature = "serde", serde(default = "MusicScore::default_beats_per_bar"))]
    pub beats_per_bar: f32,
}

impl MusicScore {
    /// Four beats to the bar.
    pub fn default_beats_per_bar() -> f32 {
        4.0
    }

    /// The layer called `name`.
    pub fn layer(&self, name: &str) -> Option<&MusicLayer> {
        self.layers.iter().find(|l| l.name == name)
    }

    /// One bar in seconds, or `None` with no beat grid.
    pub fn bar_s(&self) -> Option<f64> {
        if !self.tempo_bpm.is_finite()
            || self.tempo_bpm <= 0.0
            || !self.beats_per_bar.is_finite()
            || self.beats_per_bar <= 0.0
        {
            return None;
        }
        Some(f64::from(60.0 / self.tempo_bpm) * f64::from(self.beats_per_bar))
    }

    /// Everything wrong with the score, as sentences a person reads: a layer with no name, a layer naming a
    /// cue the table does not have, the same layer twice, a tempo or bar that is negative or not a number.
    pub fn problems(&self, table: &CueTable) -> Vec<String> {
        let mut out = Vec::new();
        if !self.tempo_bpm.is_finite() || self.tempo_bpm < 0.0 {
            out.push("tempo_bpm is negative or not a number".to_owned());
        }
        if !self.beats_per_bar.is_finite() || self.beats_per_bar <= 0.0 {
            out.push("beats_per_bar is not a positive number".to_owned());
        }
        if self.layers.is_empty() {
            out.push("the score has no layers".to_owned());
        }
        let mut seen: Vec<&str> = Vec::new();
        for layer in &self.layers {
            if layer.name.trim().is_empty() {
                out.push("a music layer has no name".to_owned());
            }
            if seen.contains(&layer.name.as_str()) {
                out.push(format!("music layer `{}` is listed more than once", layer.name));
            } else {
                seen.push(&layer.name);
            }
            match table.get(&layer.cue) {
                None => out.push(format!(
                    "music layer `{}` names cue `{}`, which the table does not have",
                    layer.name, layer.cue
                )),
                Some(cue) if !cue.mode.is_loop() => out.push(format!(
                    "music layer `{}` names cue `{}`, which is not a loop, so it cannot hold a layer",
                    layer.name, layer.cue
                )),
                Some(_) => {}
            }
        }
        out
    }
}

impl MusicScore {
    /// The score read from RON: the file a project ships and the Cauldron edits.
    #[cfg(feature = "serde")]
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("the music score does not parse: {e}"))
    }

    /// The score written as RON.
    #[cfg(feature = "serde")]
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("the music score does not write: {e}"))
    }
}

/// One layer of a score: a looping cue, and where it sits in the mix.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct MusicLayer {
    /// The layer's name, which [`MusicLayers`] moves.
    pub name: String,
    /// The looping cue this layer plays. It should name the `Music` bus.
    pub cue: String,
    /// Where the layer sits when the score starts. Most start at
    /// [`SILENT_DB`] and are brought in by gameplay.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gain_db: f32,
}

impl MusicLayer {
    /// A layer called `name` playing `cue`, silent until gameplay brings it in.
    pub fn new(name: impl Into<String>, cue: impl Into<String>) -> Self {
        Self { name: name.into(), cue: cue.into(), gain_db: SILENT_DB }
    }

    /// The same layer at `db` when the score starts.
    pub fn at_db(mut self, db: f32) -> Self {
        self.gain_db = db;
        self
    }
}

/// When a change of layers takes effect.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum When {
    /// On this frame: the change is heard at once. A stinger, or a score with no beat grid.
    #[default]
    Now,
    /// On the next beat, quantised with [`next_boundary`].
    NextBeat,
    /// On the next bar, quantised with [`next_boundary`].
    NextBar,
}

/// Move layers to new gains, on a beat or a bar.
///
/// `targets` is `(layer name, dB)`: a layer missing from the list keeps its gain. A change to `SILENT_DB` does
/// not stop the layer — it fades it out and leaves one silent voice, which is the point of vertical layering.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct MusicLayers {
    /// The layers to move, and where.
    pub targets: Vec<(String, f32)>,
    /// How long the move takes once it starts — **the time to arrive**, like [`Duck::fade_s`](crate::mix::Duck).
    /// `0` is the crate's [`MIX_TAU_S`](crate::model::MIX_TAU_S) easing (~150 ms), which is what a layer wants unless a
    /// project says otherwise; a crossfade a game asks for is usually a bar or part of one.
    pub fade_s: f32,
    /// When it starts.
    pub when: When,
}

impl MusicLayers {
    /// A change of layers, on the next bar.
    pub fn new(targets: impl IntoIterator<Item = (String, f32)>) -> Self {
        Self { targets: targets.into_iter().collect(), fade_s: 0.0, when: When::NextBar }
    }

    /// The same change, this frame.
    pub fn now(self) -> Self {
        Self { when: When::Now, ..self }
    }

    /// The same change, on the next beat.
    pub fn next_beat(self) -> Self {
        Self { when: When::NextBeat, ..self }
    }

    /// The same change, on the next bar.
    pub fn next_bar(self) -> Self {
        Self { when: When::NextBar, ..self }
    }

    /// The same change, eased over `seconds`.
    pub fn fade_s(mut self, seconds: f32) -> Self {
        self.fade_s = seconds;
        self
    }
}

/// Play a stinger over the music, ducking it while the stinger runs.
///
/// This is the one cue that arrives immediately. A stinger is a punctuation mark: it marks the moment the
/// player did something, so quantising it would miss the moment.
#[derive(Message, Clone, Debug, PartialEq)]
pub struct Stinger {
    /// The cue to play, once, on the `Music` bus.
    pub cue: String,
    /// How far the `Music` bus is lowered while the stinger plays: negative, like a duck.
    pub duck_db: f32,
    /// How long the duck holds before it starts recovering.
    pub duck_s: f32,
}

impl Stinger {
    /// A stinger of `cue` ducking the music by 6 dB for a second.
    pub fn new(cue: impl Into<String>) -> Self {
        Self { cue: cue.into(), duck_db: -6.0, duck_s: 1.0 }
    }

    /// The same stinger, ducking the music by `db`.
    pub fn duck_db(mut self, db: f32) -> Self {
        self.duck_db = db;
        self
    }

    /// The same stinger, ducking for `seconds`.
    pub fn duck_s(mut self, seconds: f32) -> Self {
        self.duck_s = seconds;
        self
    }
}

/// What the music plugin knows: which layers are live, and where each is going.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct MusicState {
    /// The score the layers were started from.
    pub score: Option<MusicScore>,
    layers: Vec<LiveLayer>,
    /// Stingers written this frame whose play has not been answered yet. A duck is a fact about a sound that **plays**,
    /// so it is armed when `Played` arrives, not when the request is written.
    pending_stingers: Vec<PendingStinger>,
    /// Cues written this frame whose [`Played`] has not been matched to a layer yet.
    awaiting: Vec<(String, String)>,
    pending: Vec<Pending>,
}

#[derive(Clone, Debug, PartialEq)]
struct LiveLayer {
    name: String,
    cue: String,
    voice: Option<Entity>,
    db: f32,
    target_db: f32,
    /// The arrival time this layer was last given, in seconds. `0` is the crate's [`MIX_TAU_S`] easing, which is what
    /// a layer starts at and what a change that names no fade leaves alone.
    fade_s: f32,
}

/// A stinger whose `Play` has been written this frame and not yet answered.
#[derive(Clone, Debug, PartialEq)]
struct PendingStinger {
    cue: String,
    duck_db: f32,
    duck_s: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct Pending {
    targets: Vec<(String, f32)>,
    at_s: f64,
    fade_s: f32,
}

impl MusicState {
    /// The gain `layer` is at this frame, or `None` for a layer the score does not have.
    pub fn layer_db(&self, layer: &str) -> Option<f32> {
        self.layers.iter().find(|l| l.name == layer).map(|l| l.db)
    }

    /// The gain `layer` is moving to.
    pub fn target_db(&self, layer: &str) -> Option<f32> {
        self.layers.iter().find(|l| l.name == layer).map(|l| l.target_db)
    }

    /// The voice entity a layer's sound is playing on, once it has started.
    pub fn voice(&self, layer: &str) -> Option<Entity> {
        self.layers.iter().find(|l| l.name == layer).and_then(|l| l.voice)
    }

    /// The layer names, in score order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.layers.iter().map(|l| l.name.as_str())
    }

    /// How many layer gains are still moving.
    pub fn moving(&self) -> usize {
        self.layers.iter().filter(|l| (l.db - l.target_db).abs() >= 0.01).count()
    }

    /// How many layer voices are live.
    pub fn live(&self) -> usize {
        self.layers.iter().filter(|l| l.voice.is_some()).count()
    }

    /// The moment the next queued change of layers is due, for a test or a panel that says "in 0.8 s".
    pub fn next_change_s(&self) -> Option<f64> {
        self.pending.iter().map(|p| p.at_s).min_by(f64::total_cmp)
    }
}

/// Marks a voice the music plugin adopted, so [`move_layers`] finds it by layer rather than by cue name.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct MusicVoice {
    /// The layer it plays.
    pub layer: String,
}

/// **Starts one looping voice per score layer** the first frame a score exists, by writing a [`Play`] per
/// layer — the crate's one play path. Runs in [`SirenSystems::Choose`](crate::SirenSystems).
pub fn start_layers(
    score: Option<Res<MusicScore>>,
    mut state: ResMut<MusicState>,
    mut plays: MessageWriter<Play>,
    mut log: ResMut<SirenLog>,
    voices: Query<(Entity, &Voice)>,
    mut commands: Commands,
    real: Res<Time<Real>>,
) {
    let Some(score) = score else {
        return;
    };
    let unchanged = !score.is_changed() && state.score.is_some();
    if unchanged {
        return;
    }
    let now = real.elapsed_secs_f64();
    let _ = now;
    state.score = Some(score.clone());

    // **A layer's voice belongs to the score's layer, and follows it.** A score is a resource a project and the
    // Cauldron edit, and this system used to only ever *add* layers: a layer that left the score kept a looping voice
    // playing forever, ownerless (`retire_voices` skips loops by design), and a layer retargeted to a new cue kept the
    // **old** sample while the new one never started. Both disagree with the score in silence.
    //
    // The stop goes through the crate's one stop path, so it fades over `DEFAULT_FADE_S` rather than clicking, and
    // `stop_requests` writes the `Stopped` line from the same reason. A **gain** edit is not a stop: that is what
    // `move_layers` moves, so the voice stays.
    let mut stops: Vec<(Entity, &'static str)> = Vec::new();
    state.layers.retain(|layer| {
        let Some(scored) = score.layer(&layer.name) else {
            if let Some(voice) = layer.voice {
                stops.push((voice, "layer removed"));
            }
            return false;
        };
        if scored.cue != layer.cue {
            if let Some(voice) = layer.voice {
                stops.push((voice, "layer retargeted"));
            }
            return false;
        }
        true
    });
    // **Marked here, not through `StopVoice`.** A `StopVoice` carries an entity and no reason, so `stop_requests`
    // would log this as a plain `"stop"` and lose what the score actually did. Marking the voice directly, with the
    // reason, keeps the one fade (`apply_stopping`) and the true log line — and `stop_requests` never sees it, so there
    // is no second line. `start_layers` runs in `Choose` and `apply_stopping` in `Mix`, so the fade begins this frame.
    for (entity, why) in stops {
        if let Ok((_, voice)) = voices.get(entity) {
            log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: why.to_owned() });
        }
        commands.entity(entity).insert(Stopping { since_s: now, why: why.to_owned() });
    }
    for layer in &score.layers {
        if state.layers.iter().all(|l| l.name != layer.name) {
            state.layers.push(LiveLayer {
                name: layer.name.clone(),
                cue: layer.cue.clone(),
                voice: None,
                db: layer.gain_db,
                target_db: layer.gain_db,
                fade_s: 0.0,
            });
        }
    }
    // **`names()` is documented as score order**, and the reconcile above drops a retargeted layer before the loop
    // re-adds it, which would put it at the end. Sort by the score's own order, with any name the score lost last.
    state.layers.sort_by_key(|layer| {
        score.layers.iter().position(|scored| scored.name == layer.name).unwrap_or(usize::MAX)
    });
    let start: Vec<(String, String, f32)> = state
        .layers
        .iter()
        .filter(|layer| layer.voice.is_none() && !state.awaiting.iter().any(|(cue, _)| *cue == layer.cue))
        .map(|layer| (layer.cue.clone(), layer.name.clone(), layer.db))
        .collect();
    for (cue, name, db) in start {
        state.awaiting.push((cue.clone(), name));
        plays.write(Play::cue(cue).gain(db));
    }
}

/// **Adopts the voices [`start_layers`] asked for**: a layer is a voice, and its gain is the layer's. Runs
/// after [`SirenSystems::Play`](crate::SirenSystems) so the [`Played`] of this frame is seen this frame.
pub fn adopt_layers(
    mut played: MessageReader<Played>,
    mut state: ResMut<MusicState>,
    mut commands: Commands,
) {
    for answer in played.read() {
        let Some(at) = state.awaiting.iter().position(|(cue, _)| *cue == answer.cue) else {
            continue;
        };
        let (cue, layer) = state.awaiting.remove(at);
        let _ = cue;
        if let Some(entry) = state.layers.iter_mut().find(|l| l.name == layer) {
            entry.voice = Some(answer.entity);
            commands.entity(answer.entity).insert(MusicVoice { layer: layer.clone() });
        }
    }
}

/// **Moves layers on the bar.** Quantises each [`MusicLayers`] to the next boundary, then eases every live
/// layer's gain toward its target — the gain the one mix system reads off the voice.
pub fn move_layers(
    mut requests: MessageReader<MusicLayers>,
    mut state: ResMut<MusicState>,
    real: Res<Time<Real>>,
    mut voices: Query<(&MusicVoice, &mut Voice)>,
) {
    let now = real.elapsed_secs_f64();
    let (tempo, beats) = state.score.as_ref().map_or((0.0, 4.0), |s| (s.tempo_bpm, s.beats_per_bar));
    for request in requests.read() {
        let at = match request.when {
            When::Now => now,
            When::NextBeat => next_boundary(now, tempo, 1.0),
            When::NextBar => next_boundary(now, tempo, f64::from(beats)),
        };
        state.pending.push(Pending { targets: request.targets.clone(), at_s: at, fade_s: request.fade_s });
    }
    let due: Vec<Pending> = state.pending.iter().filter(|p| p.at_s <= now).cloned().collect();
    state.pending.retain(|p| p.at_s > now);
    for pending in due {
        for (name, db) in pending.targets {
            if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) {
                layer.target_db = db;
                // **The fade is the layer's own**, so a change that moves three layers with three durations is
                // honoured per layer, and a layer the change does not name keeps the fade it had — exactly as it keeps
                // its gain.
                layer.fade_s = pending.fade_s;
            }
        }
    }

    let dt = real.delta_secs();
    for (music, mut voice) in &mut voices {
        let Some(layer) = state.layers.iter_mut().find(|l| l.name == music.layer) else {
            continue;
        };
        // **A frame with no elapsed time holds the gain.** `approach` returns the target for `dt <= 0`, which is right
        // for a bus that must be at its level on a game's first frame — but a layer moving over `fade_s` would then
        // skip its whole fade in the one frame that applied the change, so the number a project wrote would be spent
        // by the clock rather than by the crossfade. A layer's gain is its own state, not a level the first frame must
        // establish; the score's starting gains come from `MusicLayer::gain_db`.
        layer.db = if dt <= 0.0 || !dt.is_finite() {
            layer.db
        } else {
            let next = approach(layer.db, layer.target_db, tau_for_fade(layer.fade_s), dt);
            if (next - layer.target_db).abs() < 0.01 { layer.target_db } else { next }
        };
        voice.base_db = layer.db;
    }
}

/// **Plays a [`Stinger`]** — immediately, on the `Music` bus — and ducks the music while it runs.
pub fn play_stingers(
    mut requests: MessageReader<Stinger>,
    mut plays: MessageWriter<Play>,
    mut state: ResMut<MusicState>,
    mut log: ResMut<SirenLog>,
    mut refused: MessageWriter<crate::play::Refused>,
    real: Res<Time<Real>>,
    table: Res<CueTable>,
) {
    let now = real.elapsed_secs_f64();
    for request in requests.read() {
        // **A cue that is not in the table has no bus**, and `unwrap_or("Music")` gave it the one bus that passes the
        // check — so an unknown stinger looked like a Music cue right up to the refusal. Name it here instead.
        // **A refusal is a message, not only a log line.** A host, the editor's panel and a test all read `Refused`;
        // the log is the human-readable copy.
        let Some(cue) = table.get(&request.cue) else {
            let reason = Refusal::UnknownCue;
            log.push(Entry::Refused { at_s: now, cue: request.cue.clone(), reason: reason.to_string() });
            refused.write(crate::play::Refused { cue: request.cue.clone(), reason });
            continue;
        };
        let bus = cue.bus_name();
        if bus != "Music" {
            // **No `Refusal` variant says this**, and borrowing one would lie: the bus *is* in the mix table, it is the
            // stinger that must be on `Music`. The sentence is true and the log carries it; a host reading `Refused`
            // cannot tell this from a play it never got, which is `needs-decision/stinger-refusal.md`.
            let reason = format!("a stinger plays on Music, but `{}` names bus `{bus}`", request.cue);
            log.push(Entry::Refused { at_s: now, cue: request.cue.clone(), reason });
            continue;
        }
        plays.write(Play::cue(request.cue.clone()));
        // The duck is armed when the answer says the sound plays — `arm_stinger_ducks`, after `Play`. A duck for a
        // refused play would dip the music for a sound nobody hears; the probe for iteration 9 had exactly that:
        // `-9.00 dB` on `Music` beside `refused … no handle …` in the same log.
        state.pending_stingers.push(PendingStinger {
            cue: request.cue.clone(),
            duck_db: request.duck_db,
            duck_s: request.duck_s,
        });
    }
}

/// **Arms a stinger's duck once its play is answered**, and not before.
///
/// `DuckClock::manual` takes `(target, by_db, now, hold_s, fade_s)`, and `duck_s` is the **hold**: the music steps back
/// while the stinger sounds, then releases over the crate's `DEFAULT_FADE_S`. A project that wants its own release
/// needs a field on `Stinger`; `docs/loop/needs-decision/stinger-release.md` carries that question.
///
/// Runs after [`SirenSystems::Play`](crate::SirenSystems) and **before [`SirenSystems::Mix`](crate::SirenSystems::Mix)**,
/// so the dip is in the clock before the frame's mix reads it — a duck that landed a frame late would be a visible gap
/// between the hit and the music stepping back.
pub fn arm_stinger_ducks(
    mut played: MessageReader<Played>,
    mut refused: MessageReader<crate::play::Refused>,
    mut state: ResMut<MusicState>,
    mut clock: ResMut<DuckClock>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
) {
    if state.pending_stingers.is_empty() {
        // Still drain the answers, so this frame's `Played`/`Refused` are not re-read by a later system that expects a
        // fresh buffer.
        played.read().for_each(drop);
        refused.read().for_each(drop);
        return;
    }
    let now = real.elapsed_secs_f64();
    let answered: Vec<String> = played.read().map(|answer| answer.cue.clone()).collect();
    let turned_down: Vec<String> = refused.read().map(|answer| answer.cue.clone()).collect();
    let pending = std::mem::take(&mut state.pending_stingers);
    for stinger in pending {
        if turned_down.contains(&stinger.cue) {
            // The `Refused` line is the whole story; a `Ducked` line beside it would contradict it.
            continue;
        }
        if !answered.contains(&stinger.cue) {
            continue;
        }
        clock.manual("Music", stinger.duck_db, now, stinger.duck_s, DEFAULT_FADE_S);
        log.push(Entry::Ducked {
            at_s: now,
            bus: "Music".to_owned(),
            by_db: stinger.duck_db,
            by: "stinger".to_owned(),
        });
    }
}
