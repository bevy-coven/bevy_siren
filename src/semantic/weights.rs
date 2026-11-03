//! **How the signals are weighed, as a project's own data.**
//!
//! The ensemble combines a semantic similarity, the acoustic agreement and the penalties a filter applies. Every one of
//! those numbers is a judgement about what "fits" means, so a game whose palette is *deliberately* inconsistent — a
//! stylised world where loudness jumps are intended — must be able to say so in data rather than fork the crate.
//!
//! This follows the crate's existing shape: [`crate::cue::CueTable`] and [`crate::mix::SirenSettings`] are saved data a
//! project edits, and this is a third such type. Nothing here is a system; it is a set of numbers with a
//! [`Weights::problems`] report, so a file that says something impossible is named rather than silently obeyed.
//!
//! # Every default is a *design default*, and says why
//!
//! The crate's rule is that a number without its source, or the words **design default**, is a magic constant. None of
//! these can cite a source: what a library's sounds "fit" is a property of the library. Each carries the argument for
//! the value instead, and [`crate::semantic::eval::at_floor`] is the function that would replace them with a
//! measurement once `V-01`'s labelled set exists.
//!
//! # What is deliberately not here
//!
//! No *model* weights. The CLAP towers' numbers come from a frozen checkpoint and a project that changed them would be
//! comparing against an index built by different weights. These are the weights of the **ensemble**, which are a
//! design decision a project is entitled to make.

use std::fmt;

/// **The weights the ensemble combines signals with**, and the thresholds it filters on.
///
/// All fields are plain data so a project can ship them in its `bevy_siren.ron`, exactly as it ships a cue table. A
/// missing field takes its default, which is the behaviour that lets a file written before this type existed keep
/// loading — stated here because it is a promise and not an accident.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Weights {
    /// How much a semantic (embedding) similarity contributes.
    ///
    /// **Design default: `1.0`.** The signal the feature is named for, so it is the unit the others are relative to.
    /// A project that sets it to zero is saying *"rank my library by production only"*, which is a legitimate and
    /// unusual choice — and one `problems` permits, because it is not inconsistent.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_semantic"))]
    pub semantic: f32,

    /// How much acoustic agreement contributes — loudness, brightness and the noise floor together.
    ///
    /// **Design default: `0.5`.** Measured in the crate's own end-to-end run, the correct and incorrect semantic bands
    /// **overlap** (0.126–0.429 against −0.104–0.150), so the semantic signal alone cannot separate them. Half its
    /// weight is the smallest amount that lets production differences break a near-tie without overriding the words a
    /// person typed.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_acoustic"))]
    pub acoustic: f32,

    /// How much agreement on **tempo** contributes, when both tempi are known.
    ///
    /// **Design default: `0.35`.** Below `acoustic` because a tempo estimate carries a stated accuracy floor
    /// (`D-04`) and is absent for every effect, so a track is penalised by a number that is sometimes not there. A
    /// signal with a hole in it earns less weight than one without.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_tempo"))]
    pub tempo: f32,

    /// How much agreement on **key** contributes, when both keys are known.
    ///
    /// **Design default: `0.35`.** Equal to `tempo` and for the same reason: `D-05` detects a key with a confidence,
    /// and an atonal asset has none. Equal rather than ranked because the crate has no measurement that says which
    /// matters more, and inventing one would be a magic constant wearing a name.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_key"))]
    pub key: f32,

    /// The difference in loudness, in dB, beyond which two assets are **penalised** rather than compared.
    ///
    /// **Design default: `6.0`.** A game mix moves in steps of a few dB — the crate's own default duck is 8 dB — so two
    /// sounds more than 6 dB apart are deliberately at different levels and will not sit together.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_loudness_tolerance_db"))]
    pub loudness_tolerance_db: f32,

    /// How much noisier, in dB, a candidate's noise floor may be than the anchor's before it is **penalised**.
    ///
    /// **Design default: `12.0` dB.** Twice the loudness tolerance, because this rule is a penalty rather than an
    /// exclusion: a hissier recording is a worse neighbour, not an impossible one, and a library holds clean and noisy
    /// takes of the same instrument. 12 dB is roughly where a noise floor stops reading as "the same room" and starts
    /// reading as "a different recording" — a judgement, so it says **design default** rather than citing a source.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_noise_floor_tolerance_db"))]
    pub noise_floor_tolerance_db: f32,

    /// The share of a candidate's neighbours the ensemble drops, worst first, before a result is returned.
    ///
    /// **Design default: `0.0`.** Keep everything, because iteration 69 measured what a filter costs: at the 0.126 the
    /// correct band bottoms out at, recall@1 stays 100 %, but at 0.150 it falls to 75 % and at 0.5 to nothing. A filter
    /// that hides the right answer is worse than a long list, and there is no labelled set to choose a better fraction.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_drop_fraction"))]
    pub drop_fraction: f32,
}

impl Default for Weights {
    fn default() -> Self {
        Weights {
            semantic: Weights::default_semantic(),
            acoustic: Weights::default_acoustic(),
            tempo: Weights::default_tempo(),
            key: Weights::default_key(),
            loudness_tolerance_db: Weights::default_loudness_tolerance_db(),
            noise_floor_tolerance_db: Weights::default_noise_floor_tolerance_db(),
            drop_fraction: Weights::default_drop_fraction(),
        }
    }
}

impl Weights {
    /// **Design default: `1.0`** — the unit the other weights are relative to.
    pub fn default_semantic() -> f32 {
        1.0
    }

    /// **Design default: `0.5`** — half the semantic weight, because the semantic bands overlap and production
    /// differences are what breaks a near-tie.
    pub fn default_acoustic() -> f32 {
        0.5
    }

    /// **Design default: `0.35`** — below `acoustic`, because a tempo estimate is absent for every effect.
    pub fn default_tempo() -> f32 {
        0.35
    }

    /// **Design default: `0.35`** — equal to `tempo`, because the crate has no measurement ranking them.
    pub fn default_key() -> f32 {
        0.35
    }

    /// **Design default: `6.0` dB** — a few dB is a mix decision; more than that is a deliberate difference in level.
    pub fn default_loudness_tolerance_db() -> f32 {
        6.0
    }

    /// **Design default: `12.0` dB** — twice the loudness tolerance, because the noise rule penalises rather than
    /// excludes, and a library holds clean and noisy takes of the same instrument.
    pub fn default_noise_floor_tolerance_db() -> f32 {
        12.0
    }

    /// **Design default: `0.0`** — keep everything, because a filter that hides the right answer is worse than a long
    /// list, measured in iteration 69.
    pub fn default_drop_fraction() -> f32 {
        0.0
    }

    /// **Everything wrong with this configuration**, in the crate's own style: a list of named problems rather than a
    /// silent clamp.
    ///
    /// A configuration that cannot mean anything is refused; one that is merely unusual is not. The line is drawn at
    /// *internally inconsistent*: all weights zero ranks nothing, a negative weight subtracts a signal it was meant to
    /// add, a NaN compares false against everything, and a `drop_fraction` outside `0..1` asks for a fraction of the
    /// results that does not exist.
    pub fn problems(&self) -> Vec<WeightProblem> {
        let mut problems = Vec::new();

        // A negative or non-finite weight is not a preference, it is a sign error: it makes a signal that should attract
        // repel.
        for (name, value) in [
            ("semantic", self.semantic),
            ("acoustic", self.acoustic),
            ("tempo", self.tempo),
            ("key", self.key),
        ] {
            if !value.is_finite() {
                problems.push(WeightProblem::NotFinite { field: name, value });
            } else if value < 0.0 {
                problems.push(WeightProblem::Negative { field: name, value });
            }
        }

        // Zero across every *ranking* weight is a ranking with nothing in it. `drop_fraction` and the tolerance are not
        // weights, so they do not count towards the sum.
        if [self.semantic, self.acoustic, self.tempo, self.key].iter().all(|w| *w == 0.0) {
            problems.push(WeightProblem::AllZero);
        }

        if !self.loudness_tolerance_db.is_finite() || self.loudness_tolerance_db <= 0.0 {
            problems.push(WeightProblem::BadTolerance { value: self.loudness_tolerance_db });
        }
        // The noise-floor tolerance can be **zero** — a project that demands identical recording conditions — but not
        // negative or non-finite, which would penalise every candidate for being quieter than the anchor.
        if !self.noise_floor_tolerance_db.is_finite() || self.noise_floor_tolerance_db < 0.0 {
            problems.push(WeightProblem::BadTolerance { value: self.noise_floor_tolerance_db });
        }

        if !self.drop_fraction.is_finite() || !(0.0..1.0).contains(&self.drop_fraction) {
            problems.push(WeightProblem::BadDropFraction { value: self.drop_fraction });
        }

        problems
    }

    /// True when [`Weights::problems`] is empty.
    pub fn is_usable(&self) -> bool {
        self.problems().is_empty()
    }

    /// **The share of the total each ranking weight carries**, for a result's breakdown.
    ///
    /// A caller reporting *why* a result matched needs to say "0.42 of the score came from the words"; this is that
    /// share, normalised so the four weights sum to 1. Returns `None` when every weight is zero, which
    /// [`Weights::problems`] already refuses — the `Option` keeps the arithmetic honest rather than returning four
    /// NaNs.
    pub fn shares(&self) -> Option<Shares> {
        let total = self.semantic + self.acoustic + self.tempo + self.key;
        if total <= 0.0 || !total.is_finite() {
            return None;
        }
        Some(Shares {
            semantic: self.semantic / total,
            acoustic: self.acoustic / total,
            tempo: self.tempo / total,
            key: self.key / total,
        })
    }
}

/// Each ranking weight's share of the whole, summing to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shares {
    /// The semantic signal's share.
    pub semantic: f32,
    /// Acoustic agreement's share.
    pub acoustic: f32,
    /// Tempo agreement's share.
    pub tempo: f32,
    /// Key agreement's share.
    pub key: f32,
}

/// **Something wrong with a [`Weights`]**, named so a file that says it impossible is a message and not a silence.
#[derive(Debug, Clone, PartialEq)]
pub enum WeightProblem {
    /// A weight that is not a number compares false against everything.
    NotFinite {
        /// The field's name, as a project's file spells it.
        field: &'static str,
        /// The value found.
        value: f32,
    },
    /// A negative weight makes a signal that should attract repel.
    Negative {
        /// The field's name.
        field: &'static str,
        /// The value found.
        value: f32,
    },
    /// Every ranking weight is zero, so nothing ranks.
    AllZero,
    /// The loudness tolerance is zero or negative, so every pair is penalised.
    BadTolerance {
        /// The value found.
        value: f32,
    },
    /// The drop fraction is outside `0..1`.
    BadDropFraction {
        /// The value found.
        value: f32,
    },
}

impl fmt::Display for WeightProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeightProblem::NotFinite { field, value } => {
                write!(f, "`{field}` is {value}, which is not a number and compares false against everything")
            }
            WeightProblem::Negative { field, value } => write!(
                f,
                "`{field}` is {value}: a negative weight makes a signal that should attract repel. Remove the minus \
                 sign, or set it to 0 to ignore the signal"
            ),
            WeightProblem::AllZero => f.write_str(
                "every ranking weight is 0, so nothing would rank. Set at least one of `semantic`, `acoustic`, \
                 `tempo` or `key` above 0",
            ),
            WeightProblem::BadTolerance { value } => write!(
                f,
                "`loudness_tolerance_db` is {value}: it must be above 0, or every pair of sounds is penalised"
            ),
            WeightProblem::BadDropFraction { value } => write!(
                f,
                "`drop_fraction` is {value}: it must be at least 0 and below 1, because it is the share of results to \
                 drop"
            ),
        }
    }
}

#[cfg(feature = "serde")]
impl Weights {
    /// **Read weights from a project's RON**, with the crate's own message on a parse failure.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("the ensemble weights do not parse: {e}"))
    }

    /// **Write weights to RON**, in the form a project edits.
    pub fn to_ron(&self) -> Result<String, String> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(|e| format!("the ensemble weights do not serialise: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A negative noise-floor tolerance would penalise every candidate for being *quieter*, which is a sign error.
    #[test]
    fn a_negative_noise_floor_tolerance_is_reported() {
        let bad = Weights { noise_floor_tolerance_db: -1.0, ..Weights::default() };
        assert!(
            bad.problems().iter().any(|p| matches!(p, WeightProblem::BadTolerance { .. })),
            "a negative tolerance is a problem: {:?}",
            bad.problems()
        );
        // Zero is allowed: a project may demand identical recording conditions.
        let strict = Weights { noise_floor_tolerance_db: 0.0, ..Weights::default() };
        assert!(strict.problems().is_empty(), "{:?}", strict.problems());
    }

    #[test]
    fn the_defaults_report_nothing() {
        assert!(Weights::default().problems().is_empty(), "{:?}", Weights::default().problems());
    }
}
