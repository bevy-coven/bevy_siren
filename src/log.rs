//! **What happened, for the Cauldron's audio panel and for an agent reading the game.**
//!
//! A ring buffer, not a log file: the last [`SirenLog::CAPACITY`] decisions, each with the real time it
//! happened. A panel shows the tail; a test asserts the head; an agent that asks "why is the music quiet"
//! finds the `Ducked`/`Played`/`Refused` lines that answer it.
//!
//! Nothing here is a `println!`. A crate that prints is a crate a game cannot silence.

use std::collections::VecDeque;

use bevy::prelude::*;

/// One decision the crate made, with the moment it made it.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    /// A voice started.
    Played {
        /// When, on real time.
        at_s: f64,
        /// The cue that was asked for.
        cue: String,
        /// The variant that was chosen.
        sound: String,
        /// The bus it plays on.
        bus: String,
        /// The gain the voice started at, before the bus's own level.
        gain_db: f32,
    },
    /// A request was answered with nothing playing, and why.
    Refused {
        /// When, on real time.
        at_s: f64,
        /// The cue that was asked for.
        cue: String,
        /// The reason, as a sentence the panel can show verbatim.
        reason: String,
    },
    /// A voice stopped, and why: `"ended"`, `"stolen by footstep.grass"`, `"cue stopped"`.
    Stopped {
        /// When, on real time.
        at_s: f64,
        /// The cue whose voice stopped.
        cue: String,
        /// Why it stopped.
        why: String,
    },
    /// A duck started on a bus.
    Ducked {
        /// When, on real time.
        at_s: f64,
        /// The bus lowered.
        bus: String,
        /// How far, in dB.
        by_db: f32,
        /// What lowered it: a cue's name or `"stinger"`.
        by: String,
    },
}

impl Entry {
    /// The moment this entry happened, on real time.
    pub fn at_s(&self) -> f64 {
        match self {
            Self::Played { at_s, .. }
            | Self::Refused { at_s, .. }
            | Self::Stopped { at_s, .. }
            | Self::Ducked { at_s, .. } => *at_s,
        }
    }

    /// The cue this entry is about; a `Ducked` entry names the bus's ducking cue when it has one.
    pub fn cue(&self) -> &str {
        match self {
            Self::Played { cue, .. } | Self::Refused { cue, .. } | Self::Stopped { cue, .. } => cue,
            Self::Ducked { by, .. } => by,
        }
    }

    /// The line a panel shows: one sentence, no formatting left to the caller.
    ///
    /// **A gain is written to two decimals, because one decimal cannot show a distance law.** `-6.0206` is what
    /// `Attenuation::Inverse { near_m: 4, rolloff: 1 }` gives at 8 m and what `cues.yaml` case 14 asserts; written to one
    /// decimal it is `-6.0`, indistinguishable from the `-6.0 dB` a cue's *own* `gain_db` might author. A panel that
    /// prints `-6.02` on its cue row (iteration 41) and `-6.0` in the log is two numbers for one fact — the class this
    /// log has been fixed for at iterations 19, 31, 36 and 40. `gain_db`'s field keeps full precision; only the sentence
    /// rounds. A duck's `by_db` stays at one decimal: it is an authored depth, not a computed product.
    pub fn line(&self) -> String {
        match self {
            Self::Played { at_s, cue, sound, bus, gain_db } => {
                format!("{at_s:.2}s played `{cue}` ({sound}) on {bus} at {gain_db:.2} dB")
            }
            Self::Refused { at_s, cue, reason } => format!("{at_s:.2}s refused `{cue}`: {reason}"),
            Self::Stopped { at_s, cue, why } => format!("{at_s:.2}s stopped `{cue}`: {why}"),
            Self::Ducked { at_s, bus, by_db, by } => format!("{at_s:.2}s {by} ducked {bus} by {by_db:.1} dB"),
        }
    }
}

/// A ring buffer of the crate's recent decisions.
#[derive(Resource, Clone, Debug)]
pub struct SirenLog {
    entries: VecDeque<Entry>,
}

impl SirenLog {
    /// How many entries the buffer keeps. A panel shows eight; a test reads one; the rest is for a
    /// person asking what just happened.
    pub const CAPACITY: usize = 256;

    /// An empty log.
    pub fn new() -> Self {
        Self { entries: VecDeque::with_capacity(Self::CAPACITY) }
    }

    /// Adds `entry`, dropping one once the buffer is full.
    ///
    /// **A refusal outlives a play.** When the buffer is full, the entry dropped is the **oldest routine** one —
    /// [`Entry::Played`], [`Entry::Stopped`] or [`Entry::Ducked`] — because those happen at frame rate in a busy scene
    /// while a [`Entry::Refused`] is exceptional: a healthy game produces none. Treating them alike evicted one refusal
    /// after **256 plays** (measured), which at a 32-voice scene is about **0.4 s** — the line that explains why nothing
    /// played, thrown away by lines saying something did. If every entry is a refusal, there is nothing routine to drop
    /// and the oldest goes, because 256 refusals is a game already failing loudly.
    ///
    /// The buffer stays [`CAPACITY`](Self::CAPACITY) entries: same memory, same accessors. Only *which* entry leaves
    /// changes.
    pub fn push(&mut self, entry: Entry) {
        if self.entries.len() == Self::CAPACITY {
            match self.entries.iter().position(|e| !matches!(e, Entry::Refused { .. })) {
                Some(routine) => {
                    self.entries.remove(routine);
                }
                None => {
                    self.entries.pop_front();
                }
            }
        }
        self.entries.push_back(entry);
    }

    /// The newest `n` entries, newest first.
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &Entry> {
        self.entries.iter().rev().take(n)
    }

    /// Every entry, oldest first.
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// How many entries are held.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing has happened yet.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Empties the buffer.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl Default for SirenLog {
    fn default() -> Self {
        Self::new()
    }
}
