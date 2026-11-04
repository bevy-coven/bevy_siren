//! **Buses, ducking, the voice budget, and this frame's mix.**
//!
//! A game does not set volumes; it declares groups. A *bus* is a category — `Music`, `Effects`,
//! `Ambience`, `Dialogue` — with its own level, its own voice cap and its own mute, and the mix is the bus
//! levels plus the master, not a number per sound. That is Jason Gregory's rule (*Game Engine
//! Architecture* 3e, §14.5.8): categories and groups rather than signals, because a designer thinks "the
//! dialogue is too loud" and never "this cue is 3 dB hot". Ducking is the same idea one step further
//! (§14.6.3): a voice on one bus lowers another for a while, which is why a line of dialogue makes the
//! music step back without the game touching the music.
//!
//! **[`SirenMix`] is the whole mix, readable.** The editor's audio panel draws it, a game's own meter
//! reads it, and a test asserts on it — one number per bus, not a private float per sink.
//!
//! # The floor is one number
//!
//! [`SILENT_DB`] is where a gain becomes silence, both on the way in
//! ([`db_to_gain`]) and on the way out
//! ([`gain_to_db`](crate::model::gain_to_db)), so "is this bus audible" has one answer rather than an
//! epsilon each caller guesses.
//!
//! # The mix runs on real time
//!
//! Not on the game clock: a menu that pauses the game must not freeze a fade halfway, and a duck must
//! recover while the game is paused behind a conversation.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use crate::log::{Entry, SirenLog};
use crate::model::{MIX_TAU_S, SILENT_DB, active_duck_db, approach, chain_db, db_to_gain};
use crate::output::Output;
use crate::play::{Compensation, Completes, MixerVoice, StopCue, StopVoice, Voice};

/// The four buses every project starts with, in the order an editor lists them.
pub const DEFAULT_BUSES: [&str; 4] = ["Music", "Effects", "Ambience", "Dialogue"];

/// **A cue at or above this priority is never stolen.** Music, stingers and dialogue are the sounds a
/// player is listening *to*, so they keep their voices when effects fill the budget — Gregory §14.5.8's
/// virtual-voice rule, with the priority a cue declares.
pub const UNSTEALABLE_PRIORITY: u8 = 180;

/// The mix table: the master level, the voice budget, the buses and their ducks.
///
/// This is a [`Resource`] a game can insert, and — with the `serde` feature — a file it ships, because every
/// field is plain data with a default. The Cauldron edits the crate's own type in
/// `<project>/family/bevy_siren.ron`; there is no second schema.
#[derive(Resource, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SirenSettings {
    /// The master level, applied after every bus.
    #[cfg_attr(feature = "serde", serde(default))]
    pub master_db: f32,
    /// The most voices that may sound at once, across every bus. A voice is one playing sample, so a
    /// looping ambience bed counts for its whole life.
    #[cfg_attr(feature = "serde", serde(default = "SirenSettings::default_max_voices"))]
    pub max_voices: u16,
    /// The seed every variant pick and every detune is drawn from: the same seed replays the same
    /// sounds, which is what makes a capture or a test reproducible.
    #[cfg_attr(feature = "serde", serde(default = "SirenSettings::default_seed"))]
    pub seed: u64,
    /// The buses, in the order an editor lists them.
    #[cfg_attr(feature = "serde", serde(default = "SirenSettings::default_buses"))]
    pub buses: Vec<BusLevel>,
    /// The ducks: a voice on `source` lowers `target` while it plays.
    #[cfg_attr(feature = "serde", serde(default = "SirenSettings::default_ducks"))]
    pub ducks: Vec<Duck>,
}

/// **Every check [`SirenSettings::problems`] performs**, by name — the one list the doc and the test both read.
///
/// This exists because the count was written in **three** places: the doc's prose (*"twelve checks"*), the doc's bullet
/// list, and `tests/mix.rs::problems_names_every_mistake`'s needles. Measured: the prose said twelve, the code performed
/// **thirteen**, the test asserted **eleven**, and the check it missed was the `by_db == 0.0` one — whose own comment
/// records that it was written to fix a *measured* bug. The doc's central claim, *"this list and the code cannot drift
/// apart"*, was therefore false in the direction that matters: a maintainer adding a check would have read it, added a
/// sentence, and believed something enforced the pairing.
///
/// Ousterhout's **Information Leakage** (*A Philosophy of Software Design*, Summary of Red Flags) is exactly this — *"a
/// design decision is reflected in multiple modules"* — and the repair is to keep the decision in **one** module. The doc
/// points here; the test walks this; neither restates it.
///
/// **Adding a check means adding a name here**, and the test fails naming the one that has no needle yet.
pub const SETTINGS_CHECKS: &[&str] = &[
    "max_voices is 0",
    "master_db is not a number",
    "a bus has no name",
    "a bus is listed more than once",
    "a bus gain is not a number",
    "a bus asks for more voices than the table allows",
    "a duck starts on a bus the table does not have",
    "a duck lowers a bus the table does not have",
    "a duck raises rather than lowers",
    "a duck lowers by exactly 0 dB",
    "a duck's hold or fade is negative or not a number",
    "a duck has neither a hold nor a fade",
    "a duck starts on the bus it lowers",
];

impl SirenSettings {
    /// 32 voices: more than a busy scene's close effects, fewer than a mixer's worth of sample cost.
    pub fn default_max_voices() -> u16 {
        32
    }

    /// A fixed seed, so an audio capture is the same every run.
    pub fn default_seed() -> u64 {
        0x5EED_51AE_0000_0001
    }

    /// The four [`DEFAULT_BUSES`] at 0 dB.
    pub fn default_buses() -> Vec<BusLevel> {
        DEFAULT_BUSES.iter().map(|name| BusLevel::new(*name)).collect()
    }

    /// The one duck pair every game wants, per Gregory §14.6.3: dialogue steps the music back 8 dB and
    /// the effects 4 dB, holding a quarter second and recovering over a fraction of a second.
    pub fn default_ducks() -> Vec<Duck> {
        vec![
            Duck::new("Dialogue", "Music").by_db(-8.0).hold_s(0.25).fade_s(0.6),
            Duck::new("Dialogue", "Effects").by_db(-4.0).hold_s(0.25).fade_s(0.4),
        ]
    }

    /// The bus called `name`.
    pub fn bus(&self, name: &str) -> Option<&BusLevel> {
        self.buses.iter().find(|b| b.name == name)
    }

    /// How many voices `name` may sound: the bus's own cap, or the table's when the bus sets `0`. Never
    /// more than the table's, so a bus cannot raise the budget by itself.
    pub fn bus_voice_limit(&self, name: &str) -> usize {
        let total = usize::from(self.max_voices);
        match self.bus(name).map(|b| b.max_voices) {
            Some(0) | None => total,
            Some(cap) => usize::from(cap).min(total),
        }
    }

    /// The buses a project can name: the table's own, or the crate's four when it lists none.
    pub fn bus_names(&self) -> Vec<&str> {
        if self.buses.is_empty() {
            DEFAULT_BUSES.to_vec()
        } else {
            self.buses.iter().map(|b| b.name.as_str()).collect()
        }
    }

    /// Everything wrong with the table, as sentences a person reads on an editor's status line.
    ///
    /// **The checks are [`SETTINGS_CHECKS`], and that constant is the only list.** This doc used to enumerate them and
    /// say *"twelve checks … so this list and the code cannot drift apart"* — and it had drifted: the code performed
    /// **thirteen**, the test asserted **eleven**, and the prose said twelve. Counting in three places is Ousterhout's
    /// **Information Leakage**; the repair is one list that the doc names and the test walks.
    ///
    /// Each entry's *reason* is worth reading where it is performed — several record a **measured** bug, such as the duck
    /// that lowers by exactly `0` dB: it fires, is logged, holds for its whole `hold_s`, and never moves the bus.
    ///
    /// Two checks are **one sentence per cause, not per rule**: `default_ducks()` is `Dialogue -> Music` and
    /// `Dialogue -> Effects`, so a table without `Dialogue` has two rules with the same fault and the report says so once.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.max_voices == 0 {
            out.push("max_voices is 0, so no sound can ever play".to_owned());
        }
        if !self.master_db.is_finite() {
            out.push("master_db is not a number".to_owned());
        }
        let mut seen = BTreeSet::new();
        for bus in &self.buses {
            if bus.name.trim().is_empty() {
                out.push("a bus has no name".to_owned());
            } else if !seen.insert(bus.name.as_str()) {
                out.push(format!("bus `{}` is listed more than once", bus.name));
            }
            if !bus.gain_db.is_finite() {
                out.push(format!("bus `{}` has a gain that is not a number", bus.name));
            }
            // **A cap larger than the table's is silently the table's.** `bus_voice_limit` takes `min(cap, total)`, so
            // `BusLevel::new("Effects").max_voices(16)` under `max_voices: 32` mixes 16 and under `max_voices: 4` mixes
            // **4** — the field's doc names the `0` sentinel and not the clamp, and the guide's *"caps it further"* is
            // true without warning that a larger number is quietly reduced. A number that does nothing is the kind of
            // thing an author never discovers; say it once, where the other bus faults are.
            if bus.max_voices > 0 && bus.max_voices > self.max_voices {
                out.push(format!(
                    "bus `{}` asks for {} voices but the table's max_voices is {}, so it mixes {}",
                    bus.name, bus.max_voices, self.max_voices, self.max_voices
                ));
            }
        }
        // **The report asks `bus_names()`, like every other reader.** A table that lists no buses means the crate's
        // four - `docs/guide/buses.md` says *"everywhere - for a play's bus check, for the panel, and for the mix"* -
        // and `bus_names()` is that one answer. Building a set from `self.buses` here reported a **correctly mixed**
        // table as four missing buses, which is the worst thing a report can do: teach an author to ignore it.
        // (Iteration 10 closed the identical bypass in `update_mix`; this was the last caller doing it.)
        let names: BTreeSet<&str> = self.bus_names().into_iter().collect();
        // **One sentence per bus, however many rules name it.** `default_ducks()` is `Dialogue -> Music` and
        // `Dialogue -> Effects`, so the ordinary table has two rules with the same `source`; a panel that prints these
        // sentences verbatim showed the same fault twice and read as two faults.
        // The key is the **sentence**, not the bus: `Nope -> Music` and `Music -> Nope` are two different faults about
        // one absent bus (a duck that starts on a missing bus, and one that lowers one), and a report must keep them
        // apart. Two rules that are *the same* fault — `Dialogue -> Music` and `Dialogue -> Effects`, which is the
        // crate's own default pair — produce identical sentences and are one.
        let mut said: BTreeSet<String> = BTreeSet::new();
        for duck in &self.ducks {
            if !names.contains(duck.source.as_str()) {
                let sentence =
                    format!("a duck starts on bus `{}`, which the table does not have", duck.source);
                if said.insert(sentence.clone()) {
                    out.push(sentence);
                }
            }
            if !names.contains(duck.target.as_str()) {
                let sentence = format!("a duck lowers bus `{}`, which the table does not have", duck.target);
                if said.insert(sentence.clone()) {
                    out.push(sentence);
                }
            }
            if duck.by_db > 0.0 {
                out.push(format!(
                    "the duck on `{}` raises it by {} dB; a duck lowers, so by_db should be negative",
                    duck.target, duck.by_db
                ));
            }
            // **A rule that starts on the bus it lowers quietens its own voices.** `fire_all` matches rules on
            // `source` and `duck_db` matches them on `target`; when those are one bus, the voice that fired the rule is
            // multiplied by it too — a footstep pulling down every other effect for the rule's whole envelope, and
            // itself with them. The wording is not obviously wrong (*"when something lands on `Effects`, pull `Effects`
            // back"* is a compressor), so the sentence asks rather than forbids: a project that meant it loses one line,
            // and one that did not finds its bus quiet for a reason it could not have named.
            if duck.source == duck.target {
                out.push(format!(
                    "the duck on `{}` starts on the bus it lowers, so every voice on it quietens itself",
                    duck.target
                ));
            }
            // **A rule that lowers by zero lowers nothing, whatever its hold and fade.** `active_duck_db` returns
            // `0.0` for any `by_db >= 0.0` (`src/model.rs:187`), and `duck_db` keeps a reduction only `if db < deepest`
            // with `deepest` starting at `0.0` — so a rule at exactly `0.0` fires, is recorded in the log, holds for
            // its whole `hold_s`, and the bus never moves. A `by_db > 0.0` is already reported as a raise, and a
            // non-finite one cannot be written in RON at all (`by_db: nan` answers `Expected float`), so zero is the
            // one depth that is both authorable and silent. Reported **before** the envelope check: a `0.0` with a
            // hold and a fade has a real envelope and the wrong depth, and an author told "set hold_s or fade_s" for
            // it would fix the wrong field. Measured: `SirenSettings { ducks: [Duck::new("Dialogue","Music")
            // .by_db(0.0).hold_s(0.25).fade_s(0.6)] }.problems()` was `[]`.
            if duck.by_db == 0.0 {
                out.push(format!(
                    "the duck on `{}` lowers it by 0 dB, which is nothing at all; set by_db to the reduction you want",
                    duck.target
                ));
            }
            // **A rule with neither a hold nor a fade ducks for no time at all.** `Duck::new` leaves both at `0.0`,
            // and `active_duck_db` returns `0.0` once `elapsed >= hold` when `fade <= 0.0` — so the rule fires, is
            // recorded in the log, and is inaudible on the very frame it fires. Measured:
            // `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time` gets `0.0` from `Duck::new(..).by_db(-8.0)`
            // and `-8.0` from the same rule with the hold and fade the crate's own `default_ducks()` chains on.
            if duck.hold_s == 0.0 && duck.fade_s == 0.0 && duck.by_db < 0.0 {
                out.push(format!(
                    "the duck on `{}` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s",
                    duck.target
                ));
            }
            if !duck.hold_s.is_finite() || !duck.fade_s.is_finite() || duck.hold_s < 0.0 || duck.fade_s < 0.0
            {
                out.push(format!(
                    "the duck on `{}` has a hold or fade that is negative or not a number",
                    duck.target
                ));
            }
        }
        out
    }
}

impl SirenSettings {
    /// The mix table read from RON: the file a project ships and the Cauldron edits in
    /// `<project>/family/bevy_siren.ron`.
    #[cfg(feature = "serde")]
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("the mix table does not parse: {e}"))
    }

    /// The mix table written as RON.
    #[cfg(feature = "serde")]
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("the mix table does not write: {e}"))
    }
}

impl Default for SirenSettings {
    fn default() -> Self {
        Self {
            master_db: 0.0,
            max_voices: Self::default_max_voices(),
            seed: Self::default_seed(),
            buses: Self::default_buses(),
            ducks: Self::default_ducks(),
        }
    }
}

/// One bus: its level, its mute, and how many voices it may sound.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BusLevel {
    /// The bus's name, which a cue's `bus` field names.
    pub name: String,
    /// The bus's level, before the master and before any duck.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gain_db: f32,
    /// Mutes the bus without losing its level, so unmuting restores it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mute: bool,
    /// The most voices this bus may sound; `0` is the table's own `max_voices`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_voices: u16,
}

impl BusLevel {
    /// A bus called `name` at 0 dB, unmuted, with the table's budget.
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), gain_db: 0.0, mute: false, max_voices: 0 }
    }

    /// The same bus at `db`.
    pub fn gain_db(mut self, db: f32) -> Self {
        self.gain_db = db;
        self
    }

    /// The same bus, muted.
    pub fn muted(mut self) -> Self {
        self.mute = true;
        self
    }

    /// The same bus, capped at `voices`.
    pub fn max_voices(mut self, voices: u16) -> Self {
        self.max_voices = voices;
        self
    }
}

/// One duck: a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it recover
/// over `fade_s`.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Duck {
    /// The bus whose voices start the duck.
    pub source: String,
    /// The bus lowered while they play.
    pub target: String,
    /// How far `target` is lowered: negative, because a duck lowers.
    #[cfg_attr(feature = "serde", serde(default = "Duck::default_by_db"))]
    pub by_db: f32,
    /// How long the full reduction holds before recovery starts.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hold_s: f32,
    /// How long the recovery takes after the hold.
    #[cfg_attr(feature = "serde", serde(default))]
    pub fade_s: f32,
}

impl Duck {
    /// A duck of `target` by 6 dB with no hold and no fade: the shape a caller adjusts.
    pub fn new(source: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            by_db: Self::default_by_db(),
            hold_s: 0.0,
            fade_s: 0.0,
        }
    }

    /// −6 dB: the reduction a voice over music usually needs.
    pub fn default_by_db() -> f32 {
        -6.0
    }

    /// The same duck, lowering `target` by `db`.
    pub fn by_db(mut self, db: f32) -> Self {
        self.by_db = db;
        self
    }

    /// The same duck, holding its reduction for `seconds`.
    pub fn hold_s(mut self, seconds: f32) -> Self {
        self.hold_s = seconds;
        self
    }

    /// The same duck, recovering over `seconds`.
    pub fn fade_s(mut self, seconds: f32) -> Self {
        self.fade_s = seconds;
        self
    }
}

/// **This frame's mix**, readable by the editor and by a game's own meter.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct SirenMix {
    /// Each bus's gain this frame in dB, master included and every active duck applied.
    pub db: BTreeMap<String, f32>,
    /// Each bus's current duck in dB; `0.0` when it is not ducked. Negative means lowered.
    pub ducked: BTreeMap<String, f32>,
    /// Live voices per bus, not counting voices already fading out.
    pub voices: BTreeMap<String, usize>,
}

impl SirenMix {
    /// The gain `bus` is playing at this frame, or [`SILENT_DB`] for a bus nothing has started yet.
    pub fn bus_db(&self, bus: &str) -> f32 {
        self.db.get(bus).copied().unwrap_or(SILENT_DB)
    }

    /// **The worst-case peak `bus` can reach this frame, in dBFS, if every live voice of it were at full scale.**
    ///
    /// `n` voices summing in linear is `n` times one voice; times the bus's own gain (master and ducks included). That
    /// is an **upper bound**, not a measurement: the crate mixes samples and never reads their amplitude, so it cannot
    /// know what a recording peaks at. A bus with none of the three at unity is not at risk; a bus whose bound is above
    /// `0.0 dBFS` may clip its device, and nothing in the budget prevents it — `max_voices` is *"fewer than a mixer's
    /// worth of sample cost"* (`docs/research/siren.md`), a **sample-cost** budget, not a headroom guarantee. That is
    /// why this is a method and not a `Refusal`: the table is legal, and a game asking "can my eight effects sum past
    /// full scale" gets the arithmetic instead of guessing.
    ///
    /// ```
    /// # use bevy_siren::SirenMix;
    /// let mut mix = SirenMix::default();
    /// mix.db.insert("Effects".to_owned(), -6.0);
    /// mix.voices.insert("Effects".to_owned(), 2);
    /// // two voices at full scale, −6 dB of bus gain: +0.02 dBFS, just over the rail.
    /// assert!(mix.worst_peak_db("Effects") > 0.0);
    /// ```
    pub fn worst_peak_db(&self, bus: &str) -> f32 {
        let voices = self.voices_on(bus);
        if voices == 0 {
            // Nothing playing is not `SILENT_DB` — it is *no peak at all*, and `SILENT_DB` would read as a bus mixed
            // into the floor rather than a bus with nothing on it.
            return SILENT_DB;
        }
        // Each voice at full scale (0 dB) plus the bus's own gain, summed in linear — the same arithmetic
        // `docs/research/siren.md`'s R6 states for "several sounds sum in linear".
        let per_voice = self.bus_db(bus);
        crate::model::sum_db(&vec![per_voice; voices])
    }

    /// **The bus's worst-case peak, or `None` when nothing is on it.**
    ///
    /// [`worst_peak_db`](Self::worst_peak_db) answers [`SILENT_DB`] for a bus with no voices, and that is **the same
    /// number** a bus mixed into the floor answers with voices on it — two different facts, one value, while that
    /// method's own doc says *"nothing playing is not `SILENT_DB` — it is no peak at all"*. A meter deciding whether to
    /// draw a bar cannot tell them apart, and a game comparing `worst_peak_db(bus) > -1.0` is right by luck: an empty
    /// bus reads −80, which passes for "safe" exactly as a silent mix does.
    ///
    /// This method says it: `None` is no voice, `Some(db)` is a real bound, including one at or below the floor. The
    /// sentinel stays on `worst_peak_db` because a caller that wants one number for every bus already has it.
    pub fn peak_db(&self, bus: &str) -> Option<f32> {
        (self.voices_on(bus) > 0).then(|| self.worst_peak_db(bus))
    }

    /// How many voices `bus` has live this frame.
    pub fn voices_on(&self, bus: &str) -> usize {
        self.voices.get(bus).copied().unwrap_or(0)
    }

    /// The bus ducked furthest this frame, for a panel that says what a line of dialogue did.
    pub fn deepest_duck(&self) -> Option<(&str, f32)> {
        self.ducked
            .iter()
            .filter(|(_, db)| **db < 0.0)
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(bus, db)| (bus.as_str(), *db))
    }
}

/// When each duck last started, on real time — the envelopes the mix reads every frame.
///
/// A duck is not a volume change: it is an envelope with a start, so it lives in a resource rather than
/// being applied once to a sink.
#[derive(Resource, Default, Clone, Debug, PartialEq)]
pub struct DuckClock {
    started: BTreeMap<(String, String), f64>,
    manual: Vec<ManualDuck>,
}

#[derive(Clone, Debug, PartialEq)]
struct ManualDuck {
    target: String,
    by_db: f32,
    since_s: f64,
    hold_s: f32,
    fade_s: f32,
}

impl DuckClock {
    /// Starts the duck of `target` out of `source` at `now_s`, for a caller that knows the rule it is
    /// firing.
    pub fn fire(&mut self, source: &str, target: &str, now_s: f64) {
        self.started.insert((source.to_owned(), target.to_owned()), now_s);
    }

    /// Fires every rule whose `source` is `source`: what a starting voice on that bus does.
    pub fn fire_all(&mut self, settings: &SirenSettings, source: &str, now_s: f64) {
        for duck in settings.ducks.iter().filter(|d| d.source == source) {
            self.started.insert((duck.source.clone(), duck.target.clone()), now_s);
        }
    }

    /// Pushes a duck nobody's cue started: a stinger lowering the music it plays over.
    pub fn manual(&mut self, target: impl Into<String>, by_db: f32, now_s: f64, hold_s: f32, fade_s: f32) {
        self.manual.push(ManualDuck {
            target: target.into(),
            by_db,
            since_s: now_s,
            hold_s: hold_s.max(0.0),
            fade_s: fade_s.max(0.0),
        });
    }

    /// The strongest duck on `bus` at `now_s`, from the settings' rules and the manual envelopes. Two
    /// ducks do not stack: the deeper one wins, because a bus already at −8 dB is not lowered another
    /// 4 dB by a second voice asking for −4.
    pub fn duck_db(&self, bus: &str, settings: &SirenSettings, now_s: f64) -> f32 {
        let mut deepest = 0.0f32;
        for rule in settings.ducks.iter().filter(|d| d.target == bus) {
            let Some(start) = self.started.get(&(rule.source.clone(), rule.target.clone())) else {
                continue;
            };
            // A rule whose source bus has gone quiet was rewound by [`source_went_quiet`](Self::source_went_quiet) before
            // this read, so its fade is already underway; one whose source is still playing holds for its `hold_s`.
            let db = active_duck_db(rule.by_db, now_s - start, rule.hold_s, rule.fade_s);
            if db < deepest {
                deepest = db;
            }
        }
        for duck in self.manual.iter().filter(|d| d.target == bus) {
            let db = active_duck_db(duck.by_db, now_s - duck.since_s, duck.hold_s, duck.fade_s);
            if db < deepest {
                deepest = db;
            }
        }
        deepest
    }

    /// **Ends the hold of every rule whose source bus has gone quiet.**
    ///
    /// A duck is a rule about a **voice** — [`Duck`]'s own doc: *"a voice starting on `source` lowers `target` by
    /// `by_db` for `hold_s`, then lets it recover"* — so a duck whose source bus has **no live voice** has nothing left
    /// to hold. Measured before this: a line stopped on frame 0 left `Music` at −8 dB for another 15 frames and then
    /// spent 0.6 s recovering, ducking the music for 0.85 s after the voice was gone.
    ///
    /// The rewind is **once per duck**: `started` is moved back by the hold that had not yet elapsed, so the fade is
    /// already underway and keeps advancing. A duck that fired this frame and whose source is already quiet begins its
    /// fade now.
    pub fn source_went_quiet(
        &mut self,
        live: &BTreeMap<String, usize>,
        settings: &SirenSettings,
        now_s: f64,
    ) {
        for rule in &settings.ducks {
            if live.get(&rule.source).copied().unwrap_or(0) > 0 {
                continue;
            }
            let Some(start) = self.started.get_mut(&(rule.source.clone(), rule.target.clone())) else {
                continue;
            };
            let hold = f64::from(rule.hold_s.max(0.0));
            let elapsed = now_s - *start;
            // Only rewind when the hold has not finished: once `elapsed >= hold` the fade is running and must be left
            // alone. Rewinding again would restart it for ever, which is the bug the clamp had.
            if elapsed < hold {
                *start -= hold - elapsed;
            }
        }
    }

    /// Drops manual envelopes that have finished recovering: they are over, and keeping them would grow
    /// the list for the life of the process.
    ///
    /// **This does not touch the rules' own starts.** An entry in `started` belongs to a `(source, target)` pair, and
    /// whether that pair is still a rule is a question only the mix table can answer — so it is
    /// [`retain_rules`](Self::retain_rules), which `update_mix` calls beside this one. Measured before that split:
    /// 2000 `fire`s followed by a `prune` at t=1000 still held **2000** entries.
    pub fn prune(&mut self, now_s: f64) {
        self.manual.retain(|d| active_duck_db(d.by_db, now_s - d.since_s, d.hold_s, d.fade_s) < 0.0);
    }

    /// **Drops every rule's start that the table no longer names.**
    ///
    /// A start is kept per `(source, target)` pair and lives until it is removed, so a rule that a project edited away —
    /// a bus renamed, a duck deleted, an editor's file reloaded a hundred times — left its entry behind for the life of
    /// the process. Measured: five different source buses fired against one target, one `prune` at t=100, and
    /// `("Dialogue", "Music")` was still held; **100 table edits left 100 entries**.
    ///
    /// The pair is dropped only when no rule in `settings` names it. A manual duck is not a rule and is untouched here;
    /// it is [`prune`](Self::prune)'s.
    pub fn retain_rules(&mut self, settings: &SirenSettings) {
        self.started.retain(|(source, target), _| {
            settings.ducks.iter().any(|d| d.source == *source && d.target == *target)
        });
    }

    /// How many rule starts are held. A number a panel can draw and a test can assert, so the retained set is
    /// observable rather than inferred from `Debug`.
    pub fn rule_starts(&self) -> usize {
        self.started.len()
    }

    /// Forgets every duck: a scene change, or a test that wants a clean envelope.
    pub fn clear(&mut self) {
        self.started.clear();
        self.manual.clear();
    }
}

/// A voice whose sample is stopping, fading to silence before it is despawned.
///
/// The fade is what keeps a stolen voice from clicking. A stopping voice no longer counts against the
/// voice budget — the request that stole it proceeds in the same frame — but it is still audible for
/// [`DEFAULT_FADE_S`](crate::model::DEFAULT_FADE_S).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Stopping {
    /// When the stop was asked for, on real time.
    pub since_s: f64,
    /// Why, for the log: `"stolen by footstep.grass (cue)"`, `"stop"`, `"cue stopped"`.
    pub why: String,
}

/// Every component the mix reads off a voice: its identity, its own gain, its compensation, whether it is stopping,
/// its sink, the gain last written, and its index in the mixer (when the mixer owns playback).
pub type MixVoice<'a> = (
    Entity,
    &'a Voice,
    Option<&'a Compensation>,
    Option<&'a Stopping>,
    Option<&'a mut bevy::audio::AudioSink>,
    Option<&'a Applied>,
    Option<&'a MixerVoice>,
);

/// Every component the stop fade reads off a voice.
pub type StoppingVoice<'a> = (
    Entity,
    &'a Voice,
    &'a Stopping,
    Option<&'a mut bevy::audio::AudioSink>,
    Option<&'a Applied>,
    Option<&'a MixerVoice>,
);

/// Every component the retirement check reads off a voice: its identity, its own voice, Bevy's sink, the mixer's
/// own "ended" flag, whether it is already stopping, and its index in the mixer when that backend owns playback.
pub type EndedVoice<'a> = (
    Entity,
    &'a Voice,
    Option<&'a bevy::audio::AudioSink>,
    Option<&'a Completes>,
    Has<Stopping>,
    Option<&'a MixerVoice>,
);

/// The last gain written into a voice's sink, so the mix writes only when a value moved.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Applied {
    /// The dB last written.
    pub db: f32,
}

/// **The mix system**: ducks, bus gains, voice counters, and every voice's sink. Runs on real time.
///
/// There is exactly one writer of a voice's volume in the crate, and it is this system — which is why
/// [`MusicPlugin`](crate::MusicPlugin) moves targets and never touches a sink.
///
/// # The two backends
///
/// With `mixer` off, a voice's gain is written into its `AudioSink`, which Bevy created on the entity.
/// With `mixer` on there is no sink: the entity carries a [`MixerVoice`] and the number goes to that
/// voice's own player inside the crate's mixer, so a voice's gain and its bus's are two multiplies on the
/// audio thread rather than one on the game thread.
#[allow(clippy::too_many_arguments, unused_variables)]
pub fn update_mix(
    real: Res<Time<Real>>,
    settings: Res<SirenSettings>,
    mut clock: ResMut<DuckClock>,
    mut mix: ResMut<SirenMix>,
    mut voices: Query<MixVoice>,
    mut backend: Option<ResMut<Output>>,
    mut commands: Commands,
) {
    let now = real.elapsed_secs_f64();
    let dt = real.delta_secs();
    clock.prune(now);
    // **A start belongs to a rule, and a rule the table no longer names has no start worth keeping.** `prune` drops
    // finished manual envelopes; this drops the rule starts of a table that was edited. Without it a project that
    // reloads its mix table kept one entry per rule it had ever had — measured at 100 edits, 100 entries.
    clock.retain_rules(&settings);

    let master = if settings.master_db.is_finite() { settings.master_db } else { SILENT_DB };

    // **The buses the table has this frame** — through `bus_names()`, which is what `answer`'s `UnknownBus` check and
    // the editor's panel both read. A table that lists none means the crate's four, and iterating `settings.buses`
    // directly made that mean **no buses at all**: a cue on `Effects` was accepted and then mixed at −80 dB.
    let buses = settings.bus_names();

    // **This frame's mix has this frame's buses.** A project may swap `SirenSettings` at any time — a cutscene mix with
    // two buses, a menu with one — and a level left from the previous table is not this frame's mix: `bus_db` would
    // answer for a bus that does not exist, and `deepest_duck` could name one forever. `voices` is rebuilt wholesale a
    // few lines below; these two maps were the exception. Pruning **before** the bus loop keeps the previous value
    // readable for a bus that stayed, which is what makes a fader approach instead of step.
    mix.db.retain(|name, _| buses.contains(&name.as_str()));
    mix.ducked.retain(|name, _| buses.contains(&name.as_str()));

    // One pass over the settings: each bus's target this frame. A table that lists no buses is the crate's four, each
    // at 0 dB — the same list `bus_names()` returns, so a cue that `answer` accepts always has a bus to be mixed on.
    let levels: Vec<BusLevel> = if settings.buses.is_empty() {
        DEFAULT_BUSES.iter().map(|name| BusLevel::new(*name)).collect()
    } else {
        settings.buses.clone()
    };
    // **How many voices each bus still has, this frame, before any gain is written.** A duck is a rule about a *voice*
    // — `docs/guide/buses.md`: "a voice starting on `source` lowers `target` by `by_db` for `hold_s`" — so a duck whose
    // source bus has no live voice has nothing left to hold it. Measured before this: a line stopped on frame 0 left
    // `Music` at −8 dB for another 15 frames and then spent 0.6 s recovering, ducking the music for 0.85 s after the
    // voice was gone. The count is read here because the bus loop needs it; the write loop below recounts to decide
    // what to skip.
    let mut live: BTreeMap<String, usize> = BTreeMap::new();
    for (_, voice, _, stopping, _, _, _) in &voices {
        if stopping.is_none() {
            *live.entry(voice.bus.clone()).or_default() += 1;
        }
    }

    clock.source_went_quiet(&live, &settings, now);
    let mut targets: Vec<(String, f32)> = Vec::with_capacity(levels.len());
    for bus in &levels {
        let base = chain_db(&[master, bus.gain_db]);
        let duck = clock.duck_db(&bus.name, &settings, now);
        let target = if bus.mute || base <= SILENT_DB { SILENT_DB } else { (base + duck).max(SILENT_DB) };
        targets.push((bus.name.clone(), target));
        mix.ducked.insert(bus.name.clone(), duck);
    }

    // Each bus approaches its target. A bus nobody has written yet starts at its target rather than at
    // silence, so a game's first frame is at level rather than fading up from nothing.
    for (name, target) in &targets {
        let current = mix.db.get(name).copied().unwrap_or(*target);
        let next =
            if (target - current).abs() < 0.01 { *target } else { approach(current, *target, MIX_TAU_S, dt) };
        mix.db.insert(name.clone(), next);
        if let Some(backend) = backend.as_mut() {
            backend.0.set_bus_db(name, next);
        }
    }

    // Count the voices a player can still hear, and write each one's volume.
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (entity, voice, compensation, stopping, sink, applied, mixer_voice) in &mut voices {
        if stopping.is_some() {
            continue;
        }
        *counts.entry(voice.bus.clone()).or_default() += 1;
        let bus_db = mix.db.get(&voice.bus).copied().unwrap_or(SILENT_DB);
        let compensation_db = compensation.map_or(0.0, |c| c.db);
        // **One writer, two numbers.** Bevy's sink gets the whole chain, because the sink is the last stage before
        // the device. The crate's mixer has already summed the bus, so its voice stage wants the voice's own gain
        // and nothing else — writing the bus in too would apply it twice. Compensation divides out Bevy's panner,
        // and the mixer does not pan, so it stays out of the mixer's number.
        let written = if mixer_voice.is_some() {
            voice.base_db
        } else {
            chain_db(&[voice.base_db, compensation_db, bus_db])
        };
        if applied.is_some_and(|a| (a.db - written).abs() < 0.01) {
            continue;
        }
        match (mixer_voice, backend.as_mut()) {
            (Some(index), Some(backend)) => backend.0.set_voice_db(&voice.bus, index.0, written),
            _ => {
                if let Some(mut sink) = sink {
                    sink.set_volume(bevy::audio::Volume::Linear(db_to_gain(written)));
                }
            }
        }
        commands.entity(entity).insert(Applied { db: written });
    }
    mix.voices = counts;
}

/// **Fades a stopping voice to silence, then despawns it.** A stopping voice stops counting against the
/// budget the moment it is marked; this is the courtesy that keeps it from clicking.
#[allow(clippy::too_many_arguments, unused_variables)]
pub fn apply_stopping(
    mut commands: Commands,
    real: Res<Time<Real>>,
    mut voices: Query<StoppingVoice>,
    mut backend: Option<ResMut<Output>>,
) {
    let now = real.elapsed_secs_f64();
    let dt = real.delta_secs();
    let patience = f64::from(crate::model::DEFAULT_FADE_S) * 4.0;
    for (entity, voice, stopping, sink, applied, mixer_voice) in &mut voices {
        let current = applied.map_or(SILENT_DB, |a| a.db).min(0.0);
        let next = approach(current, SILENT_DB, MIX_TAU_S, dt);
        let over = now - stopping.since_s >= patience;
        let mut sink = sink;
        if let Some(sink) = sink.as_mut() {
            sink.set_volume(bevy::audio::Volume::Linear(db_to_gain(next)));
        }
        if let (Some(backend), Some(voice_id)) = (backend.as_mut(), mixer_voice) {
            backend.0.set_voice_db(&voice.bus, voice_id.0, next);
        }
        if next <= SILENT_DB + 0.05 || over {
            if let Some(sink) = sink.as_mut() {
                sink.stop();
            }
            if let (Some(backend), Some(voice_id)) = (backend.as_mut(), mixer_voice) {
                backend.0.stop_voice(&voice.bus, voice_id.0);
            }
            commands.entity(entity).despawn();
        }
    }
}

/// **Retires voices whose sample has ended.** A `Mode::Once` cue is a voice only while it sounds; a
/// `Mode::Loop` one lives until it is stopped, because a loop has no end to notice.
///
/// **Both backends answer.** Bevy's path says a voice ended when its `AudioSink` is empty; the crate's own mixer
/// has no sink, so it is asked through [`AudioOutput::finished`](crate::output::AudioOutput::finished) — the query the
/// backend already implements. A once-voice that is not retired keeps its slot against the budget forever, which
/// would turn a full bus into a permanent lockout rather than a steal: Gregory §14.5.8.4's voices are a fixed
/// resource, and a sound that stops *frees its resources for use by other voices*.
pub fn retire_voices(
    mut commands: Commands,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
    output: Option<ResMut<Output>>,
    voices: Query<EndedVoice>,
) {
    use bevy::audio::AudioSinkPlayback;

    let now = real.elapsed_secs_f64();
    for (entity, voice, sink, completes, stopping, mixer_voice) in &voices {
        if stopping || voice.mode.is_loop() {
            continue;
        }
        let ended = completes.is_some_and(Completes::finished)
            || sink.is_some_and(AudioSinkPlayback::empty)
            || match (output.as_ref(), mixer_voice) {
                (Some(output), Some(index)) => output.0.finished(&voice.bus, index.0),
                _ => false,
            };
        if ended {
            log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: "ended".to_owned() });
            commands.entity(entity).despawn();
        }
    }
}

/// **Handles [`StopVoice`] and [`StopCue`]**: a marked fade, never a cut.
pub fn stop_requests(
    mut stop_voice: MessageReader<StopVoice>,
    mut stop_cue: MessageReader<StopCue>,
    mut log: ResMut<SirenLog>,
    real: Res<Time<Real>>,
    voices: Query<(Entity, &Voice), Without<Stopping>>,
    mut commands: Commands,
) {
    let now = real.elapsed_secs_f64();
    let requests: Vec<(Entity, &'static str)> = stop_voice
        .read()
        .map(|r| (r.entity, "stop"))
        .chain(stop_cue.read().flat_map(|r| {
            voices.iter().filter(move |(_, v)| v.cue == r.cue).map(|(e, _)| (e, "cue stopped"))
        }))
        .collect();
    for (entity, why) in requests {
        // **Resolve against the live query first.** A [`StopVoice`] may name a voice the crate has already retired:
        // `retire_voices` despawns a one-shot the frame it ends, and a game holding [`Played::entity`] — documented as
        // the handle to keep for exactly this — has no way to know. A command on a despawned entity panics Bevy's
        // default error handler, so an ordinary race between "the player pressed the button" and "the sound finished"
        // took the game down. Stopping something that has stopped is not an error to report: there is nothing to fade
        // and nothing to log.
        //
        // The query excludes [`Stopping`], so a second stop for a voice already fading is skipped too — a fade is
        // begun once.
        let Ok((_, voice)) = voices.get(entity) else { continue };
        commands.entity(entity).insert(Stopping { since_s: now, why: why.to_owned() });
        log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: why.to_owned() });
    }
}
