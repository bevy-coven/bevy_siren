//! **Measuring a ranking, so a change to one can fail a test.**
//!
//! Everything the semantic layer decides — the floor, the ensemble weights, the chunk pooling — is a **design default**
//! because there is no labelled data to choose from. `V-01` produces that data; this module is what consumes it, and it
//! exists first because a golden set with no metric is a file, not an evaluation.
//!
//! # The two metrics, and why both
//!
//! The feature has two halves, so it gets two numbers:
//!
//! - **`recall@k`** — of the descriptions whose right answer is known, how many have it inside the first `k`. This is
//!   the number a text query lives or dies by.
//! - **`agreement`** — of the anchor pairs a person judged, how many the ranking puts on the same side. A ranking can
//!   have perfect recall and still order a library's neighbours wrongly.
//!
//! Both are reported with the **queries that failed**, by name, because a count alone tells a person nothing they can
//! act on — the same reason this crate's refusals name what is missing.
//!
//! # What this does not do
//!
//! It does not decide a threshold. It reports what a threshold *would* do: [`at_floor`] answers "if the floor
//! were this, how many right answers would survive", which is the question `V-01` exists to answer with evidence rather
//! than intuition.

use std::collections::BTreeMap;

/// One description with the asset a person says it should find.
#[derive(Debug, Clone, PartialEq)]
pub struct Labelled {
    /// The description, as a person would type it.
    pub description: String,
    /// The content hash of the asset that is the right answer.
    pub expected: u64,
}

/// One anchor and the neighbours a person says genuinely fit it.
///
/// Judgements are **positive only**: a pair that was shown and rejected is not evidence that it is wrong, because a
/// person presented with twenty candidates rejects most of them for reasons the ranking cannot know. Asserting only what
/// was affirmed keeps the metric honest at the cost of being weaker.
#[derive(Debug, Clone, PartialEq)]
pub struct Judged {
    /// The anchor's content hash.
    pub anchor: u64,
    /// The neighbours a person affirmed fit this anchor.
    pub fits: Vec<u64>,
}

/// What one ranking run produced for one description: the hashes, best first.
#[derive(Debug, Clone, PartialEq)]
pub struct Ranking {
    /// Content hashes in ranked order.
    pub order: Vec<u64>,
}

/// **Recall@k for one description**: `1.0` when the expected asset is within the first `k`, else `0.0`.
///
/// Recall rather than precision because the failure that matters here is *not finding* a sound: a wrong result in a list
/// of three costs a glance, a missing one costs a developer their afternoon.
pub fn recall_at_k(ranking: &Ranking, expected: u64, k: usize) -> f32 {
    if ranking.order.iter().take(k).any(|h| *h == expected) { 1.0 } else { 0.0 }
}

/// **Agreement for one anchor**: the share of affirmed neighbours that the ranking places above the anchor.
///
/// A neighbour that is absent from the ranking counts as **not placed**, which is a failure — the ranking did not find
/// it at all. That is deliberately not the same as "placed below", because a caller cannot tell them apart and neither
/// can a listener.
pub fn agreement(ranking: &Ranking, judged: &Judged) -> f32 {
    if judged.fits.is_empty() {
        return 1.0; // Nothing was affirmed, so nothing can be missed.
    }
    let rank_of = |hash: u64| ranking.order.iter().position(|h| *h == hash);
    let Some(anchor_at) = rank_of(judged.anchor) else {
        return 0.0; // The anchor is not in its own ranking; every judgement about it is unusable.
    };
    let hits = judged.fits.iter().filter(|&&f| rank_of(f).is_some_and(|at| at < anchor_at)).count();
    hits as f32 / judged.fits.len() as f32
}

/// **What one floor would do to a recall figure**, for choosing a floor with evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorOutcome {
    /// The floor tested.
    pub floor: f32,
    /// Descriptions whose right answer survives that floor **and** is still within `k`.
    pub kept: usize,
    /// Of how many.
    pub of: usize,
}

impl FloorOutcome {
    /// The recall this floor would report.
    pub fn recall(&self) -> f32 {
        if self.of == 0 { 0.0 } else { self.kept as f32 / self.of as f32 }
    }
}

/// **The result of running a labelled set**, with the failures named.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// Descriptions tried.
    pub descriptions: usize,
    /// Descriptions whose right answer was within `k`.
    pub recalled: usize,
    /// The descriptions that failed, with the rank the right answer actually got (or `None` when absent).
    pub missed: Vec<(String, Option<usize>)>,
    /// Anchors tried.
    pub anchors: usize,
    /// The mean agreement across anchors.
    pub mean_agreement: f32,
    /// The anchors whose agreement was below `1.0`, with their figure.
    pub disagreed: Vec<(u64, f32)>,
}

impl Report {
    /// **Recall@k** across the set: the share of descriptions that found their answer.
    pub fn recall(&self) -> f32 {
        if self.descriptions == 0 { 0.0 } else { self.recalled as f32 / self.descriptions as f32 }
    }

    /// True when every description found its answer and every anchor agreed completely.
    ///
    /// A single figure rather than two, so a test's assertion reads as one thing. A regression test that needs to know
    /// *which* half failed reads [`Report::missed`] and [`Report::disagreed`].
    pub fn perfect(&self) -> bool {
        self.missed.is_empty() && self.disagreed.is_empty() && self.descriptions > 0
    }

    /// **A sentence a person reads**, naming the failures rather than counting them.
    pub fn describe(&self) -> String {
        let recall = self.recall() * 100.0;
        if self.perfect() {
            return format!(
                "recall@k {recall:.0}% over {} descriptions, agreement 100% over {} anchors",
                self.descriptions, self.anchors
            );
        }
        let mut s = format!(
            "recall@k {recall:.0}% over {} descriptions, agreement {:.0}% over {} anchors",
            self.descriptions,
            self.mean_agreement * 100.0,
            self.anchors
        );
        if !self.missed.is_empty() {
            let names: Vec<String> = self
                .missed
                .iter()
                .map(|(d, at)| match at {
                    Some(rank) => format!("`{d}` (rank {rank})"),
                    None => format!("`{d}` (absent)"),
                })
                .collect();
            s.push_str(&format!(". Missed: {}", names.join(", ")));
        }
        if !self.disagreed.is_empty() {
            let names: Vec<String> =
                self.disagreed.iter().map(|(h, a)| format!("{h:016x} ({:.0}%)", a * 100.0)).collect();
            s.push_str(&format!(". Disagreed: {}", names.join(", ")));
        }
        s
    }
}

/// **Score a labelled set** against rankings a caller supplies.
///
/// `rankings` is keyed by description, so the metric does not own the model, the tokeniser or the index: it measures
/// whatever a caller ran. That keeps it testable with hand-written orders — which is how its own tests prove the
/// arithmetic — while the regression harness feeds it real runs.
pub fn score(
    labelled: &[Labelled],
    rankings: &BTreeMap<String, Ranking>,
    anchors: &[(Judged, Ranking)],
    k: usize,
) -> Report {
    let mut report = Report { descriptions: labelled.len(), ..Report::default() };

    for case in labelled {
        let Some(ranking) = rankings.get(&case.description) else {
            // A description with no ranking at all is a miss, named as absent: the run did not cover it.
            report.missed.push((case.description.clone(), None));
            continue;
        };
        if recall_at_k(ranking, case.expected, k) >= 1.0 {
            report.recalled += 1;
        } else {
            let at = ranking.order.iter().position(|h| *h == case.expected);
            report.missed.push((case.description.clone(), at));
        }
    }

    report.anchors = anchors.len();
    let mut total = 0.0f32;
    for (judged, ranking) in anchors {
        let a = agreement(ranking, judged);
        total += a;
        if a < 1.0 {
            report.disagreed.push((judged.anchor, a));
        }
    }
    report.mean_agreement = if anchors.is_empty() { 1.0 } else { total / anchors.len() as f32 };
    report
}

/// **What a floor would do to recall**, which is the question `V-01` exists to answer.
///
/// For each description's ranking, the right answer's score is compared with the floor: a result below the floor is
/// dropped before any caller sees it, so a floor that is too high turns a hit into a miss.
pub fn at_floor(
    labelled: &[Labelled],
    scored: &BTreeMap<String, Vec<(u64, f32)>>,
    k: usize,
    floor: f32,
) -> FloorOutcome {
    let mut kept = 0usize;
    for case in labelled {
        let Some(results) = scored.get(&case.description) else { continue };
        let surviving: Vec<u64> =
            results.iter().filter(|(_, s)| *s >= floor).map(|(h, _)| *h).take(k).collect();
        if surviving.contains(&case.expected) {
            kept += 1;
        }
    }
    FloorOutcome { floor, kept, of: labelled.len() }
}
