//! **Hard filters and penalties from the acoustic features.** The policy that turns a measurement into a decision about
//! whether two sounds belong together.
//!
//! An embedding cannot say *"this is 12 LU louder"* or *"this is in a clashing key"*. The features built since iteration
//! 77 can, and this module is where they become a **rule** rather than a term inside a scoring function — because a
//! filter that silently shapes a result is impossible to argue with, and every one here has a stated reason and a
//! stated cost.
//!
//! # The one invariant: an unknown feature never excludes
//!
//! `#27` states it, and it is the rule that makes the whole module safe:
//!
//! | Situation | Verdict |
//! |---|---|
//! | both loudnesses known, **12 LU** apart | **excluded** |
//! | both loudnesses known, **2 LU** apart | **admitted** |
//! | one loudness unknown (a clip too short to gate) | **admitted, unjudged** |
//! | key unknown on either side | **not penalised** |
//!
//! Absence is not evidence. A 0.2 s footstep has no integrated loudness at all — that is a property of the measurement,
//! not of the sound — so excluding it for failing a comparison it was never part of would silently remove the commonest
//! asset in a library. Every rule below therefore returns [`Verdict::Unjudged`] rather than a penalty when a feature is
//! missing, and [`Report`] counts those cases separately so a caller can say whether a rule judged anything at all.
//!
//! # The cascade, named
//!
//! `#27`'s own pitfall: *"when everything is filtered out, the result must say why rather than returning nothing, or the
//! tool looks broken."* [`Report::describe`] names the rule that removed the most, so an empty result is a sentence
//! rather than a mystery.

use super::key::{Estimate, Key, Relation};
use super::loudness::Loudness;
use super::spectral::Spectral;
use super::tempo::Tempo;
use super::weights::Weights;

/// What a rule decided about one candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    /// The two are compatible within this rule's threshold.
    Admitted,
    /// The rule does not apply, because one side's feature is unknown. **Never a reason to exclude.**
    Unjudged,
    /// The pair is beyond the threshold, carrying how far and by which rule.
    Excluded(Exclusion),
}

/// Why a candidate was removed, and by how much — so a caller can report a filter rather than merely apply it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exclusion {
    /// Loudness differs by more than `Weights::loudness_tolerance_db`, carrying the difference in LU.
    Loudness {
        /// The difference in dB between the two integrated loudnesses.
        difference_db: f32,
        /// The tolerance that was exceeded.
        tolerance_db: f32,
    },
    /// The tempi are farther apart than `Weights::tempo_tolerance_bpm`.
    Tempo {
        /// The difference in BPM.
        difference_bpm: f32,
        /// The tolerance that was exceeded.
        tolerance_bpm: f32,
    },
    /// The keys are a [`Relation::Clash`] — a semitone, a tritone, anything off the circle of fifths.
    ///
    /// A **penalty, never an exclusion**: a modal piece can sit anywhere, and key detection is the least reliable
    /// feature here, so `#27` puts it at the penalty level.
    Key {
        /// How the two keys relate.
        relation: Relation,
        /// The multiplier the compatibility table gives that relation.
        multiplier: f32,
    },
    /// The candidate's noise floor is so far above the anchor's that they are different recording conditions.
    ///
    /// A **penalty, not an exclusion**, by `#27`'s own scope — it is reported here so a caller can weight it, and
    /// [`evaluate`] never removes an asset for it.
    NoiseFloor {
        /// How much noisier the candidate is, in dB.
        difference_db: f32,
        /// The tolerance that was exceeded.
        tolerance_db: f32,
    },
}

impl core::fmt::Display for Exclusion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Exclusion::Loudness { difference_db, tolerance_db } => write!(
                f,
                "loudness differs by {difference_db:.1} LU, beyond the {tolerance_db:.1} LU tolerance: the two are \
                 deliberately at different levels"
            ),
            Exclusion::Tempo { difference_bpm, tolerance_bpm } => write!(
                f,
                "tempo differs by {difference_bpm:.1} BPM, beyond the {tolerance_bpm:.1} BPM tolerance: mixing them \
                 is a musical clash however well the timbres match"
            ),
            Exclusion::Key { relation, multiplier } => {
                write!(f, "{relation}, so the score is multiplied by {multiplier:.2}")
            }
            Exclusion::NoiseFloor { difference_db, tolerance_db } => write!(
                f,
                "noise floor is {difference_db:.1} dB above the anchor's, beyond {tolerance_db:.1} dB: a different \
                 recording condition"
            ),
        }
    }
}

/// **The features a filter reads.** All optional, because every one of them is absent for some real asset.
///
/// Building this from an [`crate::semantic::index::Index`] is the caller's job: the index holds them by content hash, so
/// a caller with two hashes has everything.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Features {
    /// Integrated loudness, when the clip was long enough to gate.
    pub loudness: Option<Loudness>,
    /// Spectral shape, when the analysis succeeded.
    pub spectral: Option<Spectral>,
    /// Tempo, when the clip is rhythmic enough to have one — and `Some(Tempo)` with `bpm: None` is a real answer that
    /// means *judged and not rhythmic*, which is different from `None` here.
    pub tempo: Option<Tempo>,
    /// Key and confidence, when the clip has a tonal centre.
    pub key: Option<Estimate>,
}

impl Features {
    /// The integrated loudness in LUFS, when there is one.
    pub fn lufs(&self) -> Option<f32> {
        self.loudness.and_then(|l| l.integrated)
    }

    /// The noise floor in dB, when there is one.
    pub fn noise_floor_db(&self) -> Option<f32> {
        self.spectral.map(|s| s.noise_floor_db)
    }

    /// The tempo in BPM, **only when one was actually estimated**.
    ///
    /// A record present with `bpm: None` means *the clip is not rhythmic*, and that is not a tempo: returning it would
    /// make every non-rhythmic asset compare against every other as though they shared one.
    pub fn bpm(&self) -> Option<f32> {
        self.tempo.and_then(|t| t.bpm)
    }

    /// The key, **only when one was actually found**. A record with `tonic: None` means *judged and found atonal*,
    /// which is not a key to compare.
    pub fn found_key(&self) -> Option<Key> {
        self.key.map(|e| e.key).filter(Key::is_some)
    }
}

/// **The outcome for one candidate.**
#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    /// Whether the candidate survives the hard rules.
    pub excluded: Option<Exclusion>,
    /// Penalties the caller may weigh, worst first in the order the rules ran.
    pub penalties: Vec<Exclusion>,
}

impl Judgement {
    /// True when no hard rule removed the candidate.
    pub fn admitted(&self) -> bool {
        self.excluded.is_none()
    }
}

/// **Decide whether `candidate` may sit beside `anchor`.**
///
/// Hard rules run first and can exclude; soft rules only add penalties. A feature missing on either side makes its rule
/// [`Verdict::Unjudged`] — never a reason to exclude, which is the invariant this module exists for.
pub fn evaluate(anchor: &Features, candidate: &Features, weights: &Weights) -> Judgement {
    let mut penalties = Vec::new();

    // **Loudness: a hard rule**, because a 12 LU difference is not a ranking weakness — the two cannot play together.
    if let (Some(a), Some(c)) = (anchor.lufs(), candidate.lufs()) {
        let difference_db = (c - a).abs();
        let tolerance_db = weights.loudness_tolerance_db;
        if difference_db > tolerance_db {
            return Judgement {
                excluded: Some(Exclusion::Loudness { difference_db, tolerance_db }),
                penalties,
            };
        }
    }

    // **Noise floor: a penalty, never an exclusion.** `#27` puts it at the penalty level: a noisier recording is a
    // worse neighbour, not an impossible one, and a library has clean and noisy assets of the same instrument.
    if let (Some(a), Some(c)) = (anchor.noise_floor_db(), candidate.noise_floor_db()) {
        let difference_db = c - a;
        let tolerance_db = weights.noise_floor_tolerance_db;
        if difference_db > tolerance_db {
            penalties.push(Exclusion::NoiseFloor { difference_db, tolerance_db });
        }
    }

    // **Tempo: a hard rule when both are known**, because a 90 BPM loop under a 128 BPM track is a clash whatever the
    // timbres — `#27`'s own words. A track whose tempo was never estimated, or was judged non-rhythmic, is
    // **unjudged**, never excluded.
    if let (Some(a), Some(c)) = (anchor.bpm(), candidate.bpm()) {
        let difference_bpm = (c - a).abs();
        let tolerance_bpm = weights.tempo_tolerance_bpm;
        if difference_bpm > tolerance_bpm {
            return Judgement {
                excluded: Some(Exclusion::Tempo { difference_bpm, tolerance_bpm }),
                penalties,
            };
        }
    }

    // **Key: a penalty, never an exclusion.** A `Clash` halves the score; everything else is untouched, and an absent
    // key on either side multiplies by 1.0 — the rule `Compatibility` already encodes.
    if let (Some(a), Some(c)) = (anchor.found_key(), candidate.found_key())
        && let Some(relation) = a.relation(&c)
    {
        let multiplier = weights.compatibility.multiplier(Some(relation));
        if multiplier < 1.0 {
            penalties.push(Exclusion::Key { relation, multiplier });
        }
    }

    Judgement { excluded: None, penalties }
}

/// **What a filter did to a list**, so an empty result is a sentence rather than a mystery.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// How many candidates were offered.
    pub offered: usize,
    /// How many survived.
    pub admitted: usize,
    /// How many each rule removed, in the order the rules ran.
    pub excluded_by_loudness: usize,
    /// How many carried a noise-floor penalty.
    pub penalised_for_noise: usize,
    /// How many were **unjudged** on loudness because a measurement was absent.
    ///
    /// Reported because it is the interesting number: a filter that excludes nothing on a library of short effects is
    /// working correctly *and* doing nothing, and a caller should be able to see which.
    pub unjudged_loudness: usize,
}

impl Report {
    /// **A sentence a person reads**, including the rule that removed the most — `#27`'s cascade pitfall.
    pub fn describe(&self) -> String {
        if self.offered == 0 {
            return String::from("nothing to filter");
        }
        if self.admitted == self.offered {
            return format!("all {} admitted", self.offered);
        }
        let mut s = format!("{} of {} admitted", self.admitted, self.offered);
        if self.excluded_by_loudness > 0 {
            s.push_str(&format!(", {} for loudness", self.excluded_by_loudness));
        }
        if self.admitted == 0 {
            // **The cascade case**: say why rather than returning nothing.
            s.push_str(
                ". Every candidate was removed, most by loudness: nothing in this library sits within the \
                        tolerance of the anchor",
            );
        }
        s
    }
}

/// **Apply the hard rules to a list of candidates**, with the same thresholds for each.
///
/// The anchor is not compared with itself; a caller filtering a result set passes the anchor's features and the
/// candidates' in the order it wants back.
pub fn filter(anchor: &Features, candidates: &[Features], weights: &Weights) -> (Vec<usize>, Report) {
    let mut report = Report { offered: candidates.len(), ..Report::default() };
    let mut kept = Vec::with_capacity(candidates.len());
    for (i, candidate) in candidates.iter().enumerate() {
        // Counted before the verdict, so "unjudged" reflects the measurements rather than the outcome.
        if anchor.lufs().is_none() || candidate.lufs().is_none() {
            report.unjudged_loudness += 1;
        }
        let judgement = evaluate(anchor, candidate, weights);
        match judgement.excluded {
            Some(Exclusion::Loudness { .. }) => report.excluded_by_loudness += 1,
            Some(_) => {}
            None => {
                report.admitted += 1;
                kept.push(i);
                if judgement.penalties.iter().any(|p| matches!(p, Exclusion::NoiseFloor { .. })) {
                    report.penalised_for_noise += 1;
                }
            }
        }
    }
    (kept, report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loud(lufs: f32) -> Features {
        Features {
            loudness: Some(Loudness {
                integrated: Some(lufs),
                true_peak: -1.0,
                sample_peak: -1.0,
                blocks: 7,
                blocks_possible: 7,
            }),
            spectral: None,
            tempo: None,
            key: None,
        }
    }

    fn unknown_loudness() -> Features {
        // A 0.2 s clip: no gating block, so no integrated loudness at all.
        Features {
            loudness: Some(Loudness {
                integrated: None,
                true_peak: -6.0,
                sample_peak: -6.0,
                blocks: 0,
                blocks_possible: 0,
            }),
            spectral: None,
            tempo: None,
            key: None,
        }
    }

    fn with_noise(lufs: f32, noise: f32) -> Features {
        let mut f = loud(lufs);
        f.spectral =
            Some(Spectral { centroid_hz: Some(1000.0), rolloff_hz: Some(2000.0), noise_floor_db: noise });
        f
    }

    /// **Criterion 4: a 12 LU mismatch is refused and a 2 LU one admitted**, at the default tolerance.
    #[test]
    fn twelve_lu_is_refused_and_two_lu_is_admitted() {
        let w = Weights::default();
        assert_eq!(w.loudness_tolerance_db, 6.0, "the default this test is written against");

        let far = evaluate(&loud(-20.0), &loud(-8.0), &w);
        assert!(!far.admitted(), "12 LU apart is refused");
        match far.excluded {
            Some(Exclusion::Loudness { difference_db, tolerance_db }) => {
                assert!((difference_db - 12.0).abs() < 0.01, "the difference is reported: {difference_db}");
                assert_eq!(tolerance_db, 6.0);
            }
            other => panic!("{other:?}"),
        }

        let near = evaluate(&loud(-20.0), &loud(-18.0), &w);
        assert!(near.admitted(), "2 LU apart is admitted");
        assert!(near.penalties.is_empty());
    }

    /// **Criterion 1: either side of the threshold, and the result changes across it.**
    #[test]
    fn the_loudness_threshold_is_where_the_verdict_changes() {
        let w = Weights::default();
        // Just inside, at the tolerance, and just outside.
        for (difference, should_admit) in [(5.9f32, true), (6.0, true), (6.01, false), (12.0, false)] {
            let judged = evaluate(&loud(-20.0), &loud(-20.0 + difference), &w);
            assert_eq!(
                judged.admitted(),
                should_admit,
                "a {difference} LU difference against a {} LU tolerance",
                w.loudness_tolerance_db
            );
        }
    }

    /// **Criterion 2: an unknown feature never excludes.** A short clip has no integrated loudness, so the rule cannot
    /// judge it — and must not remove it, because a 0.2 s footstep is the commonest asset in a library.
    #[test]
    fn an_unknown_loudness_never_excludes() {
        let w = Weights::default();
        // Unknown on the candidate.
        assert!(evaluate(&loud(-40.0), &unknown_loudness(), &w).admitted());
        // Unknown on the anchor.
        assert!(evaluate(&unknown_loudness(), &loud(-5.0), &w).admitted());
        // Unknown on both.
        assert!(evaluate(&unknown_loudness(), &unknown_loudness(), &w).admitted());
    }

    /// **Criterion 3: every threshold is readable and overridable from the configuration type.**
    #[test]
    fn the_tolerance_is_overridable_and_changes_the_answer() {
        let strict = Weights { loudness_tolerance_db: 1.0, ..Weights::default() };
        let loose = Weights { loudness_tolerance_db: 24.0, ..Weights::default() };
        let pair = (loud(-20.0), loud(-16.0)); // 4 LU apart

        assert!(!evaluate(&pair.0, &pair.1, &strict).admitted(), "a 1 LU tolerance refuses 4 LU");
        assert!(evaluate(&pair.0, &pair.1, &loose).admitted(), "a 24 LU tolerance admits it");
    }

    /// **The noise floor is a penalty, never an exclusion** — `#27` puts it at the penalty level.
    #[test]
    fn a_noisy_candidate_is_penalised_but_kept() {
        let w = Weights::default();
        let anchor = with_noise(-20.0, -70.0);
        let noisy = with_noise(-20.0, -30.0); // 40 dB noisier

        let judged = evaluate(&anchor, &noisy, &w);
        assert!(judged.admitted(), "a penalty never removes a candidate");
        assert_eq!(judged.penalties.len(), 1);
        match judged.penalties[0] {
            Exclusion::NoiseFloor { difference_db, .. } => assert!((difference_db - 40.0).abs() < 0.01),
            other => panic!("{other:?}"),
        }

        // And a candidate no noisier than the anchor carries nothing.
        assert!(evaluate(&anchor, &with_noise(-20.0, -70.0), &w).penalties.is_empty());
    }

    /// **The cascade, named.** When everything is removed, the report says why rather than returning nothing.
    #[test]
    fn an_all_excluded_set_says_why() {
        let w = Weights::default();
        let anchor = loud(-60.0);
        let candidates = vec![loud(-10.0), loud(-12.0), loud(-8.0)];
        let (kept, report) = filter(&anchor, &candidates, &w);

        assert!(kept.is_empty());
        assert_eq!(report.offered, 3);
        assert_eq!(report.admitted, 0);
        assert_eq!(report.excluded_by_loudness, 3);
        let sentence = report.describe();
        assert!(sentence.contains("Every candidate was removed"), "{sentence}");
        assert!(sentence.contains("loudness"), "and names the rule: {sentence}");
    }

    /// A filter that removes nothing reads as a plain admission, not as a wall of zeros.
    #[test]
    fn a_clean_set_reads_as_admitted() {
        let w = Weights::default();
        let anchor = loud(-20.0);
        let candidates = vec![loud(-18.0), loud(-22.0)];
        let (kept, report) = filter(&anchor, &candidates, &w);
        assert_eq!(kept, vec![0, 1]);
        assert_eq!(report.describe(), "all 2 admitted");
    }

    /// **Unjudged is counted separately**, because a filter that excludes nothing on a library of short effects is
    /// working correctly *and* doing nothing, and a caller should be able to see which.
    #[test]
    fn unjudged_candidates_are_counted() {
        let w = Weights::default();
        let anchor = loud(-20.0);
        let candidates = vec![loud(-18.0), unknown_loudness(), unknown_loudness()];
        let (kept, report) = filter(&anchor, &candidates, &w);
        assert_eq!(kept.len(), 3, "every candidate is admitted");
        assert_eq!(report.unjudged_loudness, 2, "and two were not judged at all");
        assert_eq!(report.describe(), "all 3 admitted");
    }

    fn with_tempo(bpm: Option<f32>) -> Features {
        Features {
            tempo: Some(Tempo { bpm, confidence: 0.8, octave_ambiguous: false }),
            ..Features::default()
        }
    }

    fn with_key(tonic: u8, major: bool) -> Features {
        Features {
            key: Some(Estimate { key: Key::new(tonic, major), confidence: 0.3 }),
            ..Features::default()
        }
    }

    /// **A tempo gap beyond the tolerance is excluded**, and one inside it is not — `#27`'s *"a pair either side of its
    /// threshold"*.
    #[test]
    fn the_tempo_threshold_is_where_the_verdict_changes() {
        let w = Weights::default();
        assert_eq!(w.tempo_tolerance_bpm, 4.0, "the default this test is written against");
        // 90 under 128 is the issue's own example: 38 BPM apart, refused.
        assert!(!evaluate(&with_tempo(Some(90.0)), &with_tempo(Some(128.0)), &w).admitted());
        // And a gap inside the tolerance is admitted.
        for (gap, should_admit) in [(0.0f32, true), (3.9, true), (4.0, true), (4.1, false), (38.0, false)] {
            let judged = evaluate(&with_tempo(Some(120.0)), &with_tempo(Some(120.0 + gap)), &w);
            assert_eq!(judged.admitted(), should_admit, "a {gap} BPM gap against a 4.0 tolerance");
        }
    }

    /// **An unknown tempo never excludes**, and neither does a *judged non-rhythmic* asset: `bpm: None` inside a record
    /// means the clip was measured and has no tempo, which is not a tempo to compare.
    #[test]
    fn an_unknown_or_non_rhythmic_tempo_never_excludes() {
        let w = Weights::default();
        let slow = with_tempo(Some(90.0));
        // Absent record, present-but-no-bpm, and present on the other side only.
        assert!(evaluate(&slow, &Features::default(), &w).admitted());
        assert!(evaluate(&slow, &with_tempo(None), &w).admitted());
        assert!(evaluate(&with_tempo(None), &slow, &w).admitted());
        assert!(evaluate(&Features::default(), &Features::default(), &w).admitted());
    }

    /// **A clashing key is penalised, never excluded**, and the multiplier comes from the table — `#27`'s criterion that
    /// the key rule works by penalty.
    #[test]
    fn a_clashing_key_is_penalised_and_never_excluded() {
        let w = Weights::default();
        // C major against C# major: a semitone apart, the clash the issue opens with.
        let judged = evaluate(&with_key(0, true), &with_key(1, true), &w);
        assert!(judged.admitted(), "a key clash never removes a candidate");
        assert_eq!(judged.penalties.len(), 1);
        match judged.penalties[0] {
            Exclusion::Key { relation, multiplier } => {
                assert_eq!(relation, Relation::Clash);
                assert!((multiplier - w.compatibility.clash_penalty).abs() < f32::EPSILON);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A consonant relationship carries no penalty at all, which is the other side of the threshold.
    #[test]
    fn a_fifth_and_a_same_key_are_unpenalised() {
        let w = Weights::default();
        assert!(evaluate(&with_key(0, true), &with_key(0, true), &w).penalties.is_empty(), "same key");
        assert!(evaluate(&with_key(0, true), &with_key(7, true), &w).penalties.is_empty(), "a fifth");
        // A parallel mode is penalised, but less than a clash.
        let parallel = evaluate(&with_key(0, true), &with_key(0, false), &w);
        assert_eq!(parallel.penalties.len(), 1);
        match parallel.penalties[0] {
            Exclusion::Key { multiplier, .. } => {
                assert!((multiplier - w.compatibility.parallel_penalty).abs() < f32::EPSILON);
                assert!(
                    multiplier > w.compatibility.clash_penalty,
                    "a parallel shift is gentler than a clash"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    /// **An unknown key never excludes and never penalises**, from either side.
    #[test]
    fn an_unknown_key_never_penalises() {
        let w = Weights::default();
        let c = with_key(0, true);
        assert!(evaluate(&c, &Features::default(), &w).penalties.is_empty());
        assert!(evaluate(&Features::default(), &c, &w).penalties.is_empty());
        // And a *judged atonal* asset is the same case: a record with no tonic is not a key to compare.
        let atonal =
            Features { key: Some(Estimate { key: Key::absent(), confidence: 0.03 }), ..Features::default() };
        assert!(evaluate(&c, &atonal, &w).penalties.is_empty());
        assert!(evaluate(&atonal, &c, &w).penalties.is_empty());
    }

    /// **Every threshold is overridable**, which is `#27`'s third criterion applied to the two new fields.
    #[test]
    fn the_new_thresholds_are_overridable() {
        let strict = Weights { tempo_tolerance_bpm: 1.0, ..Weights::default() };
        let loose = Weights { tempo_tolerance_bpm: 100.0, ..Weights::default() };
        let pair = (with_tempo(Some(120.0)), with_tempo(Some(124.0)));
        assert!(!evaluate(&pair.0, &pair.1, &strict).admitted(), "a 1 BPM tolerance refuses 4 BPM");
        assert!(evaluate(&pair.0, &pair.1, &loose).admitted(), "a 100 BPM tolerance admits it");

        // And a project that wants clashing keys sets the penalty to 1.0, which makes the rule stop penalising.
        let permissive = Weights {
            compatibility: crate::semantic::key::Compatibility { clash_penalty: 1.0, ..Default::default() },
            ..Weights::default()
        };
        assert!(
            evaluate(&with_key(0, true), &with_key(1, true), &permissive).penalties.is_empty(),
            "a clash_penalty of 1.0 is no penalty"
        );
    }

    /// The default weights report nothing, so a project that ships the defaults is not warned about them.
    #[test]
    fn the_new_defaults_are_valid() {
        assert!(Weights::default().problems().is_empty(), "{:?}", Weights::default().problems());
    }

    #[test]
    fn an_empty_set_says_there_is_nothing_to_filter() {
        let (kept, report) = filter(&loud(-20.0), &[], &Weights::default());
        assert!(kept.is_empty());
        assert_eq!(report.describe(), "nothing to filter");
    }
}
