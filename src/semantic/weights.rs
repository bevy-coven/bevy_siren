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
//! # The reference page
//!
//! Every field here, its default, its effect at its extremes and the argument for it is written up in
//! [`docs/guide/semantic-config.md`](https://github.com/bevy-coven/bevy_siren/blob/main/docs/guide/semantic-config.md),
//! which is the page a project edits against. The rustdoc below states each default beside its reason; the guide adds
//! the worked example and the thresholds' interaction.
//!
//! No *model* weights. The CLAP towers' numbers come from a frozen checkpoint and a project that changed them would be
//! comparing against an index built by different weights. These are the weights of the **ensemble**, which are a
//! design decision a project is entitled to make.

use std::fmt;

use super::key::Compatibility;

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
    /// **Design default: `0.5`** — half the semantic weight, so production differences (loudness, brightness, noise floor) can break
    /// a near-tie between candidates the words cannot tell apart, without overriding what a person typed. Not tuned: it predates the
    /// golden set, which shows one class's right answers spanning **0.27 – 0.63** in semantic score (5th to 95th percentile) — wide
    /// enough for a half-weighted acoustic term to reorder them, and silent on whether it reorders them *well*, which needs `#35`'s
    /// anchor judgements.
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

    /// How far apart two tempi may be, in BPM, before the pair is **excluded**.
    ///
    /// **Design default: `4.0` BPM.** Twice the ±2 BPM accuracy floor ADR 0003 set for the estimator, so a pair is only
    /// excluded when the difference is larger than **both** estimates' uncertainty — otherwise the rule would reject
    /// tracks whose tempi merely look different because the measurement is ±2. At 120 BPM that is ±3.3 %, below the
    /// just-noticeable difference for tempo in a rhythmic layer.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_tempo_tolerance_bpm"))]
    pub tempo_tolerance_bpm: f32,

    /// **The key compatibility table** — which relationships are consonant, and by how much the rest are penalised.
    ///
    /// `#27` requires it be a **named serde type** rather than a matrix in a system, and `#25` supplied the type. A
    /// project whose palette deliberately clashes sets `clash_penalty` to `1.0` and the rule stops penalising.
    #[cfg_attr(feature = "serde", serde(default))]
    pub compatibility: Compatibility,

    /// The share of a candidate's neighbours the ensemble drops, worst first, before a result is returned.
    ///
    /// **Design default: `0.0`.** Keep everything, because iteration 69 measured what a filter costs: at the 0.126 the
    /// correct band bottoms out at, recall@1 stays 100 %, but at 0.150 it falls to 75 % and at 0.5 to nothing. A filter
    /// that hides the right answer is worse than a long list, and there is no labelled set to choose a better fraction.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_drop_fraction"))]
    pub drop_fraction: f32,

    /// **The cosine below which a candidate is dropped before it is scored**, for the tool surface.
    ///
    /// **Design default: `0.0`**, which is [`Floor::min_score`](super::rank::Floor)'s existing default moved onto a named
    /// field rather than a second policy. `0.0` keeps everything, and **that is under review**: it was chosen from six synthetic
    /// clips, and 2 000 real recordings say a floor of about 0.35 keeps every description's top result — in the maintainer's wording
    /// and in a stranger's — while shrinking a result list ~22× (`Floor::min_score` has the numbers;
    /// `docs/loop/needs-decision/default-floor.md` has the decision).
    ///
    /// It exists because `tools::Toolset` hardcoded `Floor::default()` at its filter site: `Tally::below_floor` was
    /// documented, rendered in `describe()`, and **could never be non-zero**, and a caller had no way to set a stricter
    /// floor. A number that only the crate can choose is a number a project cannot tune.
    #[cfg_attr(feature = "serde", serde(default = "Weights::default_min_similarity"))]
    pub min_similarity: f32,
}

/// **The weighting a sound-effect query uses**, which is not the music one with numbers changed quietly.
///
/// An effect has **no key and no tempo**: those are absent, not zero, so their weights contribute nothing whenever an
/// effect library is searched — the signals would be dead weight in the table rather than doing harm, but a caller
/// reading `shares()` would be told 70 % of the score went to signals that never had a value.
///
/// So the two weights are **moved onto the signals an effect actually has**. An effect library's cohesion is timbre,
/// brightness, noise floor and loudness — which is `acoustic`, at the same 1.0 the semantic signal carries — and the
/// 0.35 + 0.35 the missing signals held is split between them, leaving tempo and key at zero because a zero there is
/// honest: the rule *would* apply, it simply has no input.
///
/// Totals are kept equal to the music default's `2.2` so a score from one weighting is comparable to the other's.
///
/// Every value here is a **design default**: there is no labelled set of effects to measure against, and `#35`'s golden
/// set does not cover them.
impl Weights {
    /// **The effects weighting**: acoustic agreement carries what key and tempo cannot.
    pub fn effects() -> Weights {
        Weights { semantic: 1.0, acoustic: 1.2, tempo: 0.0, key: 0.0, ..Weights::default() }
    }
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
            tempo_tolerance_bpm: Weights::default_tempo_tolerance_bpm(),
            compatibility: Compatibility::default(),
            drop_fraction: Weights::default_drop_fraction(),
            min_similarity: Weights::default_min_similarity(),
        }
    }
}

impl Weights {
    /// **Design default: `1.0`** — the unit the other weights are relative to.
    pub fn default_semantic() -> f32 {
        1.0
    }

    /// **Design default: `0.5`** — half the semantic weight, so production differences can break a near-tie (see `acoustic`).
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

    /// **Design default: `4.0` BPM** — twice the ±2 BPM estimator floor, so a pair is excluded only when the gap
    /// exceeds both estimates' uncertainty.
    pub fn default_tempo_tolerance_bpm() -> f32 {
        4.0
    }

    /// **Design default: `0.0`** — keep everything, because a filter that hides the right answer is worse than a long
    /// list, measured in iteration 69.
    pub fn default_drop_fraction() -> f32 {
        0.0
    }

    /// **Design default: `0.0`** — `Floor::min_score`'s default, which is **under review** (see [`Weights::min_similarity`]'s doc
    /// and `docs/loop/needs-decision/default-floor.md`).
    pub fn default_min_similarity() -> f32 {
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
        // The tempo tolerance may be zero — a project that demands exactly equal tempi — but not negative, which would
        // exclude every pair, and not non-finite, which compares false against everything.
        if !self.tempo_tolerance_bpm.is_finite() || self.tempo_tolerance_bpm < 0.0 {
            problems.push(WeightProblem::BadTolerance { value: self.tempo_tolerance_bpm });
        }
        // A multiplier above 1.0 would *reward* a clash; below 0.0 would invert the score.
        if !self.compatibility.clash_penalty.is_finite()
            || !(0.0..=1.0).contains(&self.compatibility.clash_penalty)
        {
            problems.push(WeightProblem::BadMultiplier {
                field: "clash_penalty",
                value: self.compatibility.clash_penalty,
            });
        }
        if !self.compatibility.parallel_penalty.is_finite()
            || !(0.0..=1.0).contains(&self.compatibility.parallel_penalty)
        {
            problems.push(WeightProblem::BadMultiplier {
                field: "parallel_penalty",
                value: self.compatibility.parallel_penalty,
            });
        }

        if !self.drop_fraction.is_finite() || !(0.0..1.0).contains(&self.drop_fraction) {
            problems.push(WeightProblem::BadDropFraction { value: self.drop_fraction });
        }
        // **A similarity floor beyond a cosine's range drops everything.** `-1.0..=1.0` is what a cosine can produce, so
        // `2.0` is not a stricter filter, it is an empty library that looks like a filter. The same silent-empty failure
        // the tool surface had when it could not set the floor at all.
        if !self.min_similarity.is_finite() || !(-1.0..=1.0).contains(&self.min_similarity) {
            problems.push(WeightProblem::BadSimilarity { value: self.min_similarity });
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
    /// A compatibility multiplier is outside `0..=1`.
    ///
    /// **Its own variant rather than [`WeightProblem::BadDropFraction`]**, which is what these three checks used to push —
    /// so a project that set `clash_penalty: 1.5` was told *"`drop_fraction` is 1.5"* and sent to edit a field it had not
    /// touched. The message named the wrong thing, which is the failure this crate refuses everywhere else.
    BadMultiplier {
        /// The field's name, as a project's file spells it.
        field: &'static str,
        /// The value found.
        value: f32,
    },
    /// The similarity floor is outside the range a cosine can produce.
    BadSimilarity {
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
            WeightProblem::BadMultiplier { field, value } => write!(
                f,
                "`{field}` is {value}: it multiplies a score, so it must be between 0 and 1. Above 1 rewards a clash \
                 and below 0 inverts the score"
            ),
            WeightProblem::BadSimilarity { value } => write!(
                f,
                "`min_similarity` is {value}: a cosine only reaches from -1 to 1, so this floor would drop every \
                 candidate and the library would look empty rather than filtered"
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
