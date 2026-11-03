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
use crate::play::{Play, Played, Refusal, Refused, Voice};

/// A music score: the layers a track is made of, and its tempo.
///
/// With the `serde` feature this is a file a project ships and the Cauldron edits, beside the cue table.
#[derive(Resource, Clone, Debug, PartialEq)]
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

impl Default for MusicScore {
    /// **The same score the file loader gives**, four beats to the bar.
    ///
    /// `derive(Default)` gave `beats_per_bar: 0.0` while the field carried
    /// `serde(default = "MusicScore::default_beats_per_bar")` — **4.0** — so the same score was four beats and a 2.0 s
    /// bar from RON and **zero beats with no bar at all** from `Default`, which `problems()` then reported as
    /// *"beats_per_bar is not a positive number"*: the crate's own default produced a score the crate called broken.
    /// Two spellings of one field is the fault `tests/leaf.rs` exists to prevent, and a `Default` nobody checks is where
    /// it hides.
    fn default() -> Self {
        Self { layers: Vec::new(), tempo_bpm: 0.0, beats_per_bar: Self::default_beats_per_bar() }
    }
}

/// **Every check [`MusicScore::problems`] performs**, by name — the one list the doc and the test both read.
///
/// The doc used to say *"Nine checks, and this list is the whole of them"*, and a test asserted each of the nine fires.
/// **Nothing asserted the set was exactly nine**: the nine were correct **by hand**, checked only when someone counted.
/// Adding a tenth check to `problems` left the suite green and the doc still claiming the list was the whole of it — the
/// guarantee held by luck rather than by construction.
///
/// The constant is what makes it construction: **a tenth check needs a tenth name here**, and
/// `a_score_reports_every_fault_its_doc_lists` fails naming the list when the two disagree. Proved by adding both and
/// watching it fail at `9 != 10`.
///
/// This is iteration 106's repair applied to the sibling type: Ousterhout's **Information Leakage** is one decision
/// written in several modules, and the fix is one list that the doc **names** and the test **walks**.
///
/// **Adding a check means adding a name here**, and the test fails naming the one that has no needle yet.
pub const MUSIC_CHECKS: &[&str] = &[
    "tempo is negative or not a number",
    "bar is not a positive number",
    "the score has no layers",
    "a layer has no name",
    "a layer is listed more than once",
    "a layer gain is not a number",
    "a layer names a cue the table does not have",
    "a layer names a cue that is not a loop",
    "a layer names a cue not on the Music bus",
];

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

    /// **Everything wrong with the score**, as sentences a person reads.
    ///
    /// **The checks are [`MUSIC_CHECKS`], and that constant is the only list.** This doc used to enumerate them and say
    /// *"this list is the whole of them"* while nothing asserted the set — measured: a tenth check added to this function
    /// left the suite green and the sentence still claiming completeness.
    ///
    /// [`MUSIC_CHECKS`] names each one; the reasons worth reading are where they are performed. The two the plugin also
    /// enforces (**a cue that is not a loop**, and **a cue not on the `Music` bus**) are refused at start with one line,
    /// word for word the sentence this report gives.
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
            if !layer.gain_db.is_finite() {
                out.push(format!("music layer `{}` has a gain that is not a number", layer.name));
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
                // **A layer is a stem of the `Music` bus**, and the doc said *"should"* with nothing checking. A layer
                // naming a loop on `Effects` started there: `MusicState` reported two layers live while one was mixed on
                // the effects bus. The plugin now refuses it and the report names it, in one sentence.
                Some(cue) if cue.bus_name() != "Music" => out.push(format!(
                    "music layer `{}` names cue `{}`, which plays on bus `{}`; a layer is a stem of the `Music` bus",
                    layer.name,
                    layer.cue,
                    cue.bus_name()
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
    /// The looping cue this layer plays. It **must** name the `Music` bus: a layer is a stem of that bus, and a cue on
    /// another bus is refused — not started — with a sentence in [`MusicScore::problems`] and one line in
    /// [`SirenLog`].   
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
    /// Whether a voice that dies is asked for again. **A reason the plugin did not ask for is restarted; a reason the
    /// game asked for is not.** Gregory §14.5.8.4 — a logical sound outlives its physical voice, so a stem the budget
    /// took comes back; a stem a game stopped with [`StopCue`](crate::StopCue) was meant to stop.
    restart: bool,
    /// The live voice count at the frame this layer's start was **refused**, or `None` when it was never refused. A
    /// virtual voice is promoted when a voice frees, not every frame: retrying while the budget is unchanged would
    /// write one refusal line per frame into a log a player's panel reads.
    blocked_at: Option<usize>,
    /// Whether the log has already said this layer's cue is not a loop. One line per layer, not one per frame.
    noted: bool,
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
    /// **The boundary this change was quantised to**, so a tempo edited while it waits can re-quantise it. `at_s` alone
    /// cannot: a change queued for the next bar at 160 bpm and fired at 1.5 s is the **middle** of a bar at 80 bpm, where
    /// the bar is 3.0 s — measured, and the crossfade lands off the beat.
    when: When,
    /// The tempo `at_s` was computed with, so a frame at the same tempo does no work.
    tempo: f32,
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

    /// **Whether this layer is a virtual voice** — believed playing, with no hardware voice — and the live count it
    /// waits for.
    ///
    /// `Some(n)` means the layer's `Play` was refused when `n` voices were live, and it is retried when that count
    /// falls below `n`; until then it sounds nothing. `None` means it was never refused.
    ///
    /// Gregory §14.5.8 (*Game Engine Architecture*, 3rd ed.): *"a **virtual voice** is a sound the game believes is
    /// playing but that has no hardware voice"*. A panel that draws only the layer's gain shows `-80.0 dB` for this
    /// state and `-80.0 dB` for a layer authored silent — the same string for two different answers to the one question
    /// a panel exists to answer.
    pub fn blocked(&self, layer: &str) -> Option<usize> {
        self.layers.iter().find(|l| l.name == layer).and_then(|l| l.blocked_at)
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
///
/// The argument count is Bevy's: a system's parameters are the resources it reads, and `play_requests`,
/// `answer` and `start_pending` carry the same allow for the same reason.
#[allow(clippy::too_many_arguments)]
pub fn start_layers(
    score: Option<Res<MusicScore>>,
    table: Res<CueTable>,
    mut state: ResMut<MusicState>,
    mut plays: MessageWriter<Play>,
    mut log: ResMut<SirenLog>,
    voices: LayerVoices,
    mut commands: Commands,
    real: Res<Time<Real>>,
) {
    let Some(score) = score else {
        return;
    };
    let now = real.elapsed_secs_f64();
    let live = &voices;
    // **The mirror is reconciled every frame**, not only when the score changes. `MusicState` records the entity a
    // layer's `Played` reported, and a voice can die for a reason the score never heard about: the budget steals it
    // (§ R4), a game stops it (`StopCue`), a one-shot it named ends. Nothing in the plugin noticed, so `voice()` kept
    // naming a **despawned** entity, `live()` counted it, and the layer was silent for the rest of the process.
    reconcile(&mut state, live);
    if !score.is_changed() && state.score.is_some() {
        start_missing(&mut state, &mut plays, &mut log, &table, live, now);
        return;
    }
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
            // **A retarget re-arms a layer a game had stopped.** The layer is dropped here and re-created by the loop
            // below with `restart: true`, because the score now names a *different* sound, which is a stem the game
            // asked for. Keeping the old `restart: false` would re-add the layer and then never start it — exactly the
            // silent disagreement this iteration exists to remove.
            return false;
        }
        true
    });
    // **Marked here, not through `StopVoice`.** A `StopVoice` carries an entity and no reason, so `stop_requests`
    // would log this as a plain `"stop"` and lose what the score actually did. Marking the voice directly, with the
    // reason, keeps the one fade (`apply_stopping`) and the true log line — and `stop_requests` never sees it, so there
    // is no second line. `start_layers` runs in `Choose` and `apply_stopping` in `Mix`, so the fade begins this frame.
    for (entity, why) in stops {
        if let Ok((_, voice, _, _)) = voices.get(entity) {
            log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: why.to_owned() });
        }
        commands.entity(entity).insert(Stopping { since_s: now, why: why.to_owned() });
    }
    for layer in &score.layers {
        if state.layers.iter().all(|l| l.name != layer.name) {
            // **A layer's starting gain is a caller value like any other.** Iteration 18 gave the crate one rule for a
            // number that is not a number - it is the neutral one for what it measures - and applied it to the four
            // numbers entering `answer`. A layer's gain entered here instead: `NaN` went straight into `db`,
            // `target_db` and `Voice.base_db`, `chain_db` silenced the sink (correctly), and **nothing said so** - a
            // silent stem with a fader at `NaN`. `SILENT_DB` is finite, so a layer a score starts silent is untouched;
            // this only catches a value that is not a number at all.
            let start_db = crate::model::finite_or(layer.gain_db, 0.0);
            state.layers.push(LiveLayer {
                name: layer.name.clone(),
                cue: layer.cue.clone(),
                voice: None,
                db: start_db,
                target_db: start_db,
                fade_s: 0.0,
                restart: true,
                blocked_at: None,
                noted: false,
            });
        }
    }
    // **`names()` is documented as score order**, and the reconcile above drops a retargeted layer before the loop
    // re-adds it, which would put it at the end. Sort by the score's own order, with any name the score lost last.
    state.layers.sort_by_key(|layer| {
        score.layers.iter().position(|scored| scored.name == layer.name).unwrap_or(usize::MAX)
    });
    start_missing(&mut state, &mut plays, &mut log, &table, live, now);
}

/// The voices the plugin may see, for the reconcile and for the budget it must not fill.
type LayerVoices<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static crate::play::Voice, Option<&'static MusicVoice>, Option<&'static Stopping>),
>;

/// **Clears a layer's voice when the voice has died**, and remembers whether it should come back.
fn reconcile(state: &mut MusicState, live: &LayerVoices) {
    let mut alive: Vec<Entity> = Vec::new();
    for (entity, voice, _, _) in live.iter() {
        let _ = voice;
        alive.push(entity);
    }
    for layer in &mut state.layers {
        let Some(voice) = layer.voice else { continue };
        if alive.contains(&voice) {
            // Still in the world: read **why** it is going while the reason is still there. `Stopping` is removed with
            // the entity, so a frame later the question is unanswerable.
            if let Ok((_, _, _, Some(stopping))) = live.get(voice)
                && is_game_stop(&stopping.why)
            {
                layer.restart = false;
            }
            continue;
        }
        layer.voice = None;
        if !layer.restart {
            // The game asked for this stop, so the layer stays stopped. It is **silent**, and `layer_db` must say so:
            // a panel reading a fader that no voice is following would be reading the gain of the last frame the layer
            // played, which is not this frame's music.
            layer.db = crate::model::SILENT_DB;
            layer.target_db = crate::model::SILENT_DB;
        }
    }
}

/// Whether a [`Stopping`] reason is the game's, rather than the crate's own housekeeping.
fn is_game_stop(why: &str) -> bool {
    why == "stop" || why == "cue stopped"
}

/// **Asks for a voice for every layer that has none and may have one.** A refused start is retried **when something
/// made room** — the live count fell below the count at the refusal — because a budget that has not changed will refuse
/// identically, and writing that line every frame is noise in the panel the log exists for.
fn start_missing(
    state: &mut MusicState,
    plays: &mut MessageWriter<Play>,
    log: &mut SirenLog,
    table: &CueTable,
    live: &LayerVoices,
    now: f64,
) {
    let count = live.iter().count();
    // **A layer starts a voice its cue says will not end.** `MusicScore::problems` states the rule - *"names cue
    // `hit`, which is not a loop, so it cannot hold a layer"* - and this system used to ignore it: a `Mode::Once` cue
    // was started, its voice ended, and the restart rule asked for it again **every frame** (measured: 10 plays in 10
    // frames, no `Refused`, no line). An unknown cue is refused with a reason; a one-shot cue was played at frame rate
    // and never mentioned. The check reads the cue the layer names **this frame**, so a table edit that breaks a live
    // layer stops its restart, and one that fixes it starts the layer on the next frame.
    // Each entry is `(layer name, cue, the sentence)`: the notice loop writes one line per layer, and the reason is
    // carried rather than assumed, so a second kind of fault can reuse the same machinery.
    let mut bad: Vec<(String, String, String)> = Vec::new();
    let start: Vec<(String, String, f32)> = state
        .layers
        .iter()
        .filter(|layer| {
            if layer.voice.is_some() || !layer.restart {
                return false;
            }
            let Some(cue) = table.get(&layer.cue) else {
                // A missing cue has its own path: `play_requests` refuses it with `UnknownCue` and the log says why.
                return !state.awaiting.iter().any(|(cue, _)| *cue == layer.cue)
                    && layer.blocked_at.is_none_or(|blocked| count < blocked);
            };
            if !cue.mode.is_loop() {
                bad.push((
                    layer.name.clone(),
                    layer.cue.clone(),
                    format!(
                        "music layer `{}` names cue `{}`, which is not a loop, so it cannot hold a layer",
                        layer.name, layer.cue
                    ),
                ));
                return false;
            }
            // **A layer is a stem of the `Music` bus.** `MusicLayer::cue`'s doc says it *"should name the `Music` bus"*, and
            // nothing checked: a layer naming a loop on `Effects` started there, so `MusicState` said two layers were live
            // while one was mixed on the effects bus — the score and the sound disagreeing, which `problems()` did not
            // report either. The same fault a stinger already refuses to play on (see `play_stingers`).
            if cue.bus_name() != "Music" {
                bad.push((
                    layer.name.clone(),
                    layer.cue.clone(),
                    format!(
                        "music layer `{}` names cue `{}`, which plays on bus `{}`; a layer is a stem of the `Music` bus",
                        layer.name,
                        layer.cue,
                        cue.bus_name()
                    ),
                ));
                return false;
            }
            !state.awaiting.iter().any(|(cue, _)| *cue == layer.cue)
                && layer.blocked_at.is_none_or(|blocked| count < blocked)
        })
        .map(|layer| (layer.cue.clone(), layer.name.clone(), layer.db))
        .collect();
    for (name, cue, reason) in bad {
        let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) else {
            continue;
        };
        if layer.noted {
            continue;
        }
        layer.noted = true;
        // One line per layer, in the words `MusicScore::problems` already uses, so a panel and the report agree.
        log.push(Entry::Refused { at_s: now, cue: cue.clone(), reason });
    }
    for (cue, name, db) in start {
        if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) {
            layer.blocked_at = None;
        }
        state.awaiting.push((cue.clone(), name));
        plays.write(Play::cue(cue).gain(db));
    }
}

/// **Adopts the voices [`start_layers`] asked for**: a layer is a voice, and its gain is the layer's. Runs
/// after [`SirenSystems::Play`](crate::SirenSystems) so the [`Played`] of this frame is seen this frame.
pub fn adopt_layers(
    mut played: MessageReader<Played>,
    mut refused: MessageReader<Refused>,
    mut state: ResMut<MusicState>,
    voices: LayerVoices,
    mut commands: Commands,
) {
    // How many voices the world holds **when the answer arrived**, which is the number a retry must fall below: at a
    // refusal the budget is full, so the same count would refuse identically.
    let live_count = voices.iter().count();
    // **A refusal is an answer too.** `awaiting` is the intent to start a layer, and an intent that produced a
    // `Refused` is spent: leaving it there blocked the layer for the life of the process, because `start_layers` skips
    // a layer whose cue is already awaiting. Clear it and record the count it was refused at, so the retry waits for a
    // voice to free rather than writing the same refusal into the panel's log every frame.
    for answer in refused.read() {
        let Some(at) = state.awaiting.iter().position(|(cue, _)| *cue == answer.cue) else {
            continue;
        };
        let (_, name) = state.awaiting.remove(at);
        if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) {
            layer.blocked_at = Some(live_count);
        }
    }
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
    mut log: ResMut<SirenLog>,
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
        state.pending.push(Pending {
            targets: request.targets.clone(),
            at_s: at,
            fade_s: request.fade_s,
            when: request.when,
            tempo,
        });
    }
    // **A queued change follows the score's tempo.** A `MusicScore` is a resource a project and the Cauldron edit, and a
    // tempo change while a `NextBar` change waits made it land mid-bar: queued at 160 bpm for 1.5 s, fired at 1.5 s after
    // the score said 80, where the bar is 3.0 s. Re-quantise against the current tempo, keeping the same boundary kind,
    // and never move it **earlier** than the frame that is reading it.
    for pending in &mut state.pending {
        if pending.tempo != tempo && pending.at_s > now {
            let at = match pending.when {
                When::Now => pending.at_s,
                When::NextBeat => next_boundary(now, tempo, 1.0),
                When::NextBar => next_boundary(now, tempo, f64::from(beats)),
            };
            pending.at_s = at.max(now);
            pending.tempo = tempo;
        }
    }
    let due: Vec<Pending> = state.pending.iter().filter(|p| p.at_s <= now).cloned().collect();
    state.pending.retain(|p| p.at_s > now);
    for pending in due {
        for (name, db) in pending.targets {
            // **A target the score cannot name is said, not dropped.** `find` returned `None` and the change vanished:
            // no log line, no refusal, nothing — so a typo in a layer name was a change that silently did nothing, which
            // is the class of bug this crate's refusals exist to prevent. `MusicLayers` is a message with no
            // `Refusal` arm (`needs-decision/` has no file for one, and adding an enum variant is the same break three
            // other open decisions wait on), so the sentence goes where iteration 9 put the stinger's wrong-bus case:
            // the log, naming the layer the author typed. The change still does nothing — this reports, it does not
            // play anything differently.
            let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) else {
                let reason = format!("this score has no layer `{name}`, so the change names nothing");
                log.push(Entry::Refused { at_s: now, cue: name.clone(), reason });
                continue;
            };
            // The same rule for a move: a target that is not a number is **no gain**, not `NaN` written into a
            // layer that is already moving (`approach(NaN, x)` returns `x`, but `layer_db` would report `NaN` for
            // the frames before the move lands).
            layer.target_db = crate::model::finite_or(db, 0.0);
            // **The fade is the layer's own**, so a change that moves three layers with three durations is
            // honoured per layer, and a layer the change does not name keeps the fade it had — exactly as it keeps
            // its gain.
            layer.fade_s = pending.fade_s;
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
        // **A stinger that cannot dip does not claim it did.** `active_duck_db` returns `0.0` for any
        // `by_db >= 0.0`, so a `duck_db` of exactly `0.0` — or a positive one, meant as a lift — armed an envelope
        // that lowers nothing while this line said `Ducked { by_db: 0.0 }`. A `Stinger` is a message with no
        // `problems()`, so the log was the only place a person could have seen it, and it said the opposite.
        // Iteration 52 fixed the same inert path for a table rule; this is the message that reaches it from the other
        // side. Measured: `Stinger { duck_db: 0.0, .. }` logged `Ducked { by_db: 0.0 }`.
        if stinger.duck_db.is_finite() && stinger.duck_db < 0.0 {
            clock.manual("Music", stinger.duck_db, now, stinger.duck_s, DEFAULT_FADE_S);
            log.push(Entry::Ducked {
                at_s: now,
                bus: "Music".to_owned(),
                by_db: stinger.duck_db,
                by: "stinger".to_owned(),
            });
        }
    }
}
