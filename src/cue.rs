//! **A cue table as data**: what a game can play, by name, and what each name means.
//!
//! A *cue* is the thing gameplay asks for — `"footstep.grass"`, `"ui.cancel"`, `"music.combat"` — and it
//! is deliberately not a file. A cue has *variants* (several recordings of the same event, so a player
//! does not hear one sample sixty times), a bus, a gain, a pitch range, a repeat rule, a polyphony
//! limit, a priority and a spatial model. The sound files are named separately, in
//! [`SirenSounds`](crate::sounds::SirenSounds), so a cue can be written before an artist delivers anything and
//! a delivered file can replace another without touching the table.
//!
//! This is the shape Simon Franco's *Artificial Sound Designer* argues for (Game AI Pro, ch. 46): a
//! play *request* carries a type and its subject, and the system decides which sample and how loud.
//! That is why [`Cue::sounds`] is a list and why [`PitchRange`] exists — the variation is data, not a
//! special case in the caller.
//!
//! **Everything here is a value.** Nothing in this file allocates on a play path: the systems index the
//! table, never build one.

use crate::mix::DEFAULT_BUSES;

/// One cue: a name gameplay asks for, and everything about how it sounds.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Cue {
    /// The name gameplay asks for: `"footstep.grass"`, `"ui.cancel"`.
    pub name: String,
    /// The recordings this cue may play. Empty means the cue can only refuse
    /// ([`CueProblem::NoSounds`]).
    #[cfg_attr(feature = "serde", serde(default))]
    pub sounds: Vec<SoundSlot>,
    /// The bus this cue plays on. `""` is the crate's `Effects` bus, the one a project gets for free.
    #[cfg_attr(feature = "serde", serde(default))]
    pub bus: String,
    /// The cue's own level, before its variant's, the request's and the bus's.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gain_db: f32,
    /// How much every play is detuned, so repeats of one recording do not sound mechanical.
    #[cfg_attr(feature = "serde", serde(default))]
    pub pitch: PitchRange,
    /// How the next variant is chosen.
    #[cfg_attr(feature = "serde", serde(default))]
    pub repeat: Repeat,
    /// Whether one play ends or loops.
    #[cfg_attr(feature = "serde", serde(default))]
    pub mode: Mode,
    /// How many of this cue may **start** at once; `0` is no per-cue limit, which
    /// [`CueProblem::UnlimitedInstances`] reports because it is almost never what a project meant.
    ///
    /// **A budget on plays, not a standing count of the voices sounding.** The cap decides whether the *next* play may
    /// start, and a voice it admits lives out its length. A table edited while voices are playing — the ordinary path,
    /// since a [`CueTable`] is a resource a game may replace at any time — does not end them: two voices of a cue whose
    /// `max_instances` was just lowered to one keep sounding, and the next play is refused with
    /// [`VoiceLimit`](crate::Refusal). Ending them would cut a sound on a frame the game asked for nothing, which is a
    /// worse surprise than the one it would fix.
    #[cfg_attr(feature = "serde", serde(default = "Cue::one"))]
    pub max_instances: u16,
    /// Which cue keeps its voice when the budget is full: higher survives.
    #[cfg_attr(feature = "serde", serde(default))]
    pub priority: u8,
    /// Whether the cue is placed in the world. A spatial cue played with
    /// [`Play::at`](crate::Play::at) attenuates with distance; one played without a position is heard
    /// by everyone at the cue's own gain.
    #[cfg_attr(feature = "serde", serde(default))]
    pub spatial: bool,
    /// How fast a spatial cue falls off with distance.
    #[cfg_attr(feature = "serde", serde(default))]
    pub attenuation: Attenuation,
}

impl Cue {
    /// A cue named `name` with no variants: the shape an editor adds first.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sounds: Vec::new(),
            bus: String::new(),
            gain_db: 0.0,
            pitch: PitchRange::default(),
            repeat: Repeat::default(),
            mode: Mode::default(),
            max_instances: Self::one(),
            priority: 0,
            spatial: false,
            attenuation: Attenuation::default(),
        }
    }

    /// The bus this cue plays on, with `""` resolved to the crate's `Effects` bus.
    pub fn bus_name(&self) -> &str {
        if self.bus.is_empty() { "Effects" } else { &self.bus }
    }

    /// `max_instances`' default: one at a time, the answer for a footstep, a click or a line.
    pub fn one() -> u16 {
        1
    }

    /// True when the cue can be played at all: it has a variant.
    pub fn has_sounds(&self) -> bool {
        !self.sounds.is_empty()
    }
}

/// One recording a cue may play, with its own trim.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SoundSlot {
    /// The key [`SirenSounds`](crate::sounds::SirenSounds) maps to a file.
    pub sound: String,
    /// A trim for this recording, so variants mastered at different levels play at one level.
    #[cfg_attr(feature = "serde", serde(default))]
    pub gain_db: f32,
}

impl SoundSlot {
    /// A slot for `sound` at its recorded level.
    pub fn new(sound: impl Into<String>) -> Self {
        Self { sound: sound.into(), gain_db: 0.0 }
    }

    /// The same slot, trimmed by `db`.
    pub fn gain_db(mut self, db: f32) -> Self {
        self.gain_db = db;
        self
    }
}

/// The range a cue's pitch is drawn from, in semitones.
///
/// The default, ±1 semitone, is small on purpose: enough that four plays of one footstep are four
/// sounds, little enough that no one notices the pitch moving.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PitchRange {
    /// The lowest detune, inclusive.
    #[cfg_attr(feature = "serde", serde(default = "PitchRange::default_low"))]
    pub low: f32,
    /// The highest detune, inclusive.
    #[cfg_attr(feature = "serde", serde(default = "PitchRange::default_high"))]
    pub high: f32,
}

impl PitchRange {
    /// −1 semitone.
    pub fn default_low() -> f32 {
        -1.0
    }

    /// +1 semitone.
    pub fn default_high() -> f32 {
        1.0
    }

    /// A fixed pitch.
    pub fn exact(semitones: f32) -> Self {
        Self { low: semitones, high: semitones }
    }

    /// True when `low` is past `high`: [`CueProblem::InvertedPitch`].
    pub fn is_inverted(&self) -> bool {
        !self.low.is_finite() || !self.high.is_finite() || self.low > self.high
    }

    /// A detune in `[low, high]`, from `unit` in `[0, 1)`.
    pub fn at(&self, unit: f32) -> f32 {
        if self.is_inverted() {
            return 0.0;
        }
        self.low + (self.high - self.low) * unit.clamp(0.0, 1.0)
    }
}

impl Default for PitchRange {
    fn default() -> Self {
        Self { low: Self::default_low(), high: Self::default_high() }
    }
}

/// How a cue picks its next variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Repeat {
    /// Random, but never one of the last `avoid` variants — the default, and what kills the
    /// machine-gun footstep.
    AvoidRepeat {
        /// How many recent picks are refused.
        avoid: u8,
    },
    /// A plain random draw, repeats included: a burst weapon wants the same sample.
    Shuffle,
}

impl Default for Repeat {
    fn default() -> Self {
        Self::AvoidRepeat { avoid: 1 }
    }
}

impl Repeat {
    /// How many recent picks this rule refuses: `Shuffle` refuses none.
    pub fn avoid(&self) -> u8 {
        match self {
            Self::AvoidRepeat { avoid } => *avoid,
            Self::Shuffle => 0,
        }
    }
}

/// Whether one play of a cue ends, or repeats until it is stopped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Mode {
    /// Plays once. The voice goes away when it ends.
    #[default]
    Once,
    /// Repeats until stopped: a wind loop, an engine, a music stem.
    Loop,
}

impl Mode {
    /// True for [`Mode::Loop`].
    pub fn is_loop(self) -> bool {
        matches!(self, Self::Loop)
    }
}

/// How a spatial cue's gain falls off with distance.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Attenuation {
    /// No falloff: a cue heard everywhere, like a radio or a voice in the player's earpiece.
    None,
    /// A physical point source: full gain inside `near_m`, then `near / (near + rolloff * (d - near))`.
    /// This is the default, with 4 m and rolloff 1.
    Inverse {
        /// The radius at which the cue starts to fall off, in metres.
        #[cfg_attr(feature = "serde", serde(default = "Attenuation::default_near_m"))]
        near_m: f32,
        /// How fast it falls off past `near_m`; 1 is a point source in free air.
        #[cfg_attr(feature = "serde", serde(default = "Attenuation::default_rolloff"))]
        rolloff: f32,
    },
    /// Exactly silent at and past `max_m`, with a straight line between.
    Linear {
        /// The distance at which the cue is silent, in metres.
        max_m: f32,
    },
}

impl Attenuation {
    /// 4 m: the distance at which a footstep stops being at the listener's feet.
    pub fn default_near_m() -> f32 {
        4.0
    }

    /// 1.0: an inverse-distance point source.
    pub fn default_rolloff() -> f32 {
        1.0
    }

    /// True when the constants cannot produce a gain: a non-finite or negative radius, rolloff or
    /// distance.
    pub fn is_bad(&self) -> bool {
        match self {
            Self::None => false,
            Self::Inverse { near_m, rolloff } => {
                !near_m.is_finite() || !rolloff.is_finite() || *near_m <= 0.0 || *rolloff < 0.0
            }
            Self::Linear { max_m } => !max_m.is_finite() || *max_m <= 0.0,
        }
    }
}

impl Default for Attenuation {
    fn default() -> Self {
        Self::Inverse { near_m: Self::default_near_m(), rolloff: Self::default_rolloff() }
    }
}

/// One thing wrong with a cue table, as a sentence a person reads on the editor's status line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CueProblem {
    /// A cue with no variant can only refuse every play.
    NoSounds {
        /// The cue's name.
        name: String,
    },
    /// Two cues share a name, so which one plays is a lookup detail rather than a decision.
    DuplicateName {
        /// The repeated name.
        name: String,
    },
    /// A cue names a bus the mix table does not have.
    UnknownBus {
        /// The cue's name.
        name: String,
        /// The bus it names.
        bus: String,
    },
    /// [`PitchRange::low`] is past [`PitchRange::high`], so the range is empty.
    InvertedPitch {
        /// The cue's name.
        name: String,
    },
    /// An attenuation constant is negative or not a number, so the distance gain is undefined.
    BadAttenuation {
        /// The cue's name.
        name: String,
    },
    /// `max_instances` is `0`: no per-cue limit, which the whole-table budget still bounds.
    UnlimitedInstances {
        /// The cue's name.
        name: String,
    },
}

impl std::fmt::Display for CueProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSounds { name } => write!(f, "cue `{name}` has no sounds, so it can only be refused"),
            Self::DuplicateName { name } => write!(f, "cue `{name}` is defined more than once"),
            Self::UnknownBus { name, bus } => {
                write!(f, "cue `{name}` plays on bus `{bus}`, which the mix table does not have")
            }
            Self::InvertedPitch { name } => {
                write!(f, "cue `{name}` has pitch.low past pitch.high, so no detune can be drawn")
            }
            Self::BadAttenuation { name } => {
                write!(f, "cue `{name}` has an attenuation constant that is negative or not a number")
            }
            Self::UnlimitedInstances { name } => {
                write!(
                    f,
                    "cue `{name}` sets max_instances 0, which is no per-cue limit; only the table's max_voices bounds it"
                )
            }
        }
    }
}

/// Every cue a game can play, by name.
#[derive(bevy::prelude::Resource, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CueTable {
    /// The cues, in the order a person wrote them (the editor shows that order).
    #[cfg_attr(feature = "serde", serde(default))]
    pub cues: Vec<Cue>,
}

impl CueTable {
    /// A table of `cues`.
    pub fn new(cues: Vec<Cue>) -> Self {
        Self { cues }
    }

    /// The cue called `name`.
    pub fn get(&self, name: &str) -> Option<&Cue> {
        self.cues.iter().find(|c| c.name == name)
    }

    /// Adds `cue`, replacing a cue of the same name and returning the one it replaced.
    ///
    /// Voices already playing the replaced cue are **not** ended: its `max_instances` governs whether the next play may
    /// start, so lowering it under live voices leaves them sounding and refuses the next request. See
    /// [`Cue::max_instances`].
    pub fn insert(&mut self, cue: Cue) -> Option<Cue> {
        match self.cues.iter_mut().find(|c| c.name == cue.name) {
            Some(slot) => Some(std::mem::replace(slot, cue)),
            None => {
                self.cues.push(cue);
                None
            }
        }
    }

    /// Removes the cue called `name`, returning it.
    pub fn remove(&mut self, name: &str) -> Option<Cue> {
        let at = self.cues.iter().position(|c| c.name == name)?;
        Some(self.cues.remove(at))
    }

    /// Every cue's name, in table order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.cues.iter().map(|c| c.name.as_str())
    }

    /// Every distinct `sound` key the table names, in table order: what a project has to supply.
    pub fn sounds(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for slot in self.cues.iter().flat_map(|c| c.sounds.iter()) {
            if !out.contains(&slot.sound.as_str()) {
                out.push(&slot.sound);
            }
        }
        out
    }

    /// Everything wrong with the table against the crate's own four buses
    /// ([`DEFAULT_BUSES`]).
    pub fn problems(&self) -> Vec<CueProblem> {
        self.problems_with(&DEFAULT_BUSES)
    }

    /// Everything wrong with the table against `buses`, the mix table's own list.
    pub fn problems_with(&self, buses: &[&str]) -> Vec<CueProblem> {
        let mut out = Vec::new();
        for (i, cue) in self.cues.iter().enumerate() {
            if cue.sounds.is_empty() {
                out.push(CueProblem::NoSounds { name: cue.name.clone() });
            }
            if self.cues[..i].iter().any(|other| other.name == cue.name) {
                out.push(CueProblem::DuplicateName { name: cue.name.clone() });
            }
            if !buses.contains(&cue.bus_name()) {
                out.push(CueProblem::UnknownBus { name: cue.name.clone(), bus: cue.bus_name().to_owned() });
            }
            if cue.pitch.is_inverted() {
                out.push(CueProblem::InvertedPitch { name: cue.name.clone() });
            }
            if cue.attenuation.is_bad() {
                out.push(CueProblem::BadAttenuation { name: cue.name.clone() });
            }
            if cue.max_instances == 0 {
                out.push(CueProblem::UnlimitedInstances { name: cue.name.clone() });
            }
        }
        out
    }

    /// Whether every cue's `sound` key has a handle in `sounds`.
    pub fn missing(&self, sounds: &crate::SirenSounds) -> Vec<String> {
        sounds.missing(self)
    }

    /// The table read from RON, or a sentence saying what is wrong with it.
    #[cfg(feature = "serde")]
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("the cue table does not parse: {e}"))
    }

    /// The table written as RON, the form the Cauldron edits in `<project>/family/bevy_siren.ron`.
    #[cfg(feature = "serde")]
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("the cue table does not write: {e}"))
    }
}
