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
//!
//! # A class of right answers
//!
//! The functions above score a description whose right answer is **one** asset. A golden set's queries name a **class**, and a
//! class has many members — a text encoder cannot tell twelve takes of one bark apart, and a metric that demanded a particular one
//! would measure the corpus's order rather than the ranking. [`ScoredCase`] carries a query's whole ranking and its **set** of right
//! answers, and [`score_cases`], [`floor_curve`] and [`score_bands`] are the three questions put to it: *did it find one*, *what
//! would a floor keep and silence*, and *how far apart are the scores of the right answers and the wrong ones*.

use std::collections::{BTreeMap, BTreeSet};

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

// ---------------------------------------------------------------------------------------------------------------------
// A class of right answers: what a golden set's text queries are scored by.
// ---------------------------------------------------------------------------------------------------------------------

/// The cut-off the report names its misses at: **3**, `#29`'s workflow — *describe, see three, pick one* (ADR 0007).
///
/// The other cut-offs a report states, 1 and 10, are the ends of the range the model's own paper reports (R@1, R@5, R@10 and
/// mAP@10; DOI `10.1109/icassp49357.2023.10095969`).
pub const REPORT_K: usize = 3;

/// **One text query run against a whole library**: what was asked, which assets are right answers, and **every** asset's
/// similarity to the description, best first.
///
/// The right answers are a **set**. An **empty** set is a query that should find *nothing* — a description of a sound the
/// library does not contain — which is how a similarity floor is shown to do its job rather than only to cost recall.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCase {
    /// The description, as a person would type it.
    pub description: String,
    /// The content hashes that are right answers. Empty: nothing in this library should match.
    pub right: BTreeSet<u64>,
    /// Every asset and its cosine similarity to the description, **best first** — what [`crate::semantic::rank::top_k`]
    /// returns when it is asked for all of them. The functions here read it as sorted.
    pub scored: Vec<(u64, f32)>,
}

impl ScoredCase {
    /// True when nothing in the library is a right answer: a query that should find nothing.
    pub fn is_negative(&self) -> bool {
        self.right.is_empty()
    }

    /// The 1-based rank of the first right answer, or `None` when no right answer is in `scored` at all.
    pub fn first_right(&self) -> Option<usize> {
        self.scored.iter().position(|(hash, _)| self.right.contains(hash)).map(|at| at + 1)
    }

    /// **Hit@k**: is any right answer within the first `k`? A prefix test — a right answer at rank 9 is not found by `k = 3`.
    ///
    /// The number a text query lives or dies by: a wrong result in a short list costs a glance, a missing right one costs a
    /// developer their afternoon.
    pub fn hit(&self, k: usize) -> bool {
        self.scored.iter().take(k).any(|(hash, _)| self.right.contains(hash))
    }

    /// **Precision@k**: the share of the first `k` that are right. A library with fewer than `k` assets still divides by `k`: a
    /// short list is not a precise one.
    pub fn precision(&self, k: usize) -> f32 {
        if k == 0 {
            return 0.0;
        }
        let hits = self.scored.iter().take(k).filter(|(hash, _)| self.right.contains(hash)).count();
        hits as f32 / k as f32
    }

    /// **Reciprocal rank**: `1 / rank` of the first right answer, and `0.0` when there is none.
    pub fn reciprocal_rank(&self) -> f32 {
        self.first_right().map_or(0.0, |rank| 1.0 / rank as f32)
    }

    /// **Average precision@k**: the mean of the precision at each right answer inside the first `k`, over the most right answers
    /// those `k` could hold (`min(k, |right|)`). `1.0` when the first `min(k, |right|)` results are all right, `0.0` when none is.
    pub fn average_precision(&self, k: usize) -> f32 {
        let possible = k.min(self.right.len());
        if possible == 0 {
            return 0.0;
        }
        let (mut hits, mut sum) = (0usize, 0.0f32);
        for (at, (hash, _)) in self.scored.iter().take(k).enumerate() {
            if self.right.contains(hash) {
                hits += 1;
                sum += hits as f32 / (at + 1) as f32;
            }
        }
        sum / possible as f32
    }
}

/// **What a golden set's positive queries scored**, with the misses named.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SetScores {
    /// Queries that have a right answer. A query that should find nothing is not in this count.
    pub queries: usize,
    /// The share of queries with a right answer in the first result.
    pub hit_1: f32,
    /// … in the first three ([`REPORT_K`]).
    pub hit_3: f32,
    /// … in the first ten.
    pub hit_10: f32,
    /// The mean share of the first ten results that are right.
    pub precision_10: f32,
    /// The mean reciprocal rank of the first right answer.
    pub mrr: f32,
    /// The mean of [`ScoredCase::average_precision`] at 10.
    pub map_10: f32,
    /// The queries with no right answer in the first [`REPORT_K`], with the rank their first right answer actually got
    /// (`None` when none is in the ranking at all). In input order.
    pub missed: Vec<(String, Option<usize>)>,
}

/// **Score the queries that have right answers.** Queries that should find nothing are not scored here — [`floor_curve`] and
/// [`score_bands`] are what they are for.
pub fn score_cases(cases: &[ScoredCase]) -> SetScores {
    let positives: Vec<&ScoredCase> = cases.iter().filter(|case| !case.is_negative()).collect();
    if positives.is_empty() {
        return SetScores::default();
    }
    let n = positives.len() as f32;
    let mean =
        |per_case: fn(&ScoredCase) -> f32| positives.iter().map(|case| per_case(case)).sum::<f32>() / n;
    SetScores {
        queries: positives.len(),
        hit_1: mean(|case| f32::from(case.hit(1))),
        hit_3: mean(|case| f32::from(case.hit(REPORT_K))),
        hit_10: mean(|case| f32::from(case.hit(10))),
        precision_10: mean(|case| case.precision(10)),
        mrr: mean(ScoredCase::reciprocal_rank),
        map_10: mean(|case| case.average_precision(10)),
        missed: positives
            .iter()
            .filter(|case| !case.hit(REPORT_K))
            .map(|case| (case.description.clone(), case.first_right()))
            .collect(),
    }
}

/// **What one similarity floor does to a golden set**: what it keeps of the right answers and what it silences of the wrong.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloorRow {
    /// The floor tested: results scoring below it are dropped.
    pub floor: f32,
    /// Of the queries that have a right answer: the share whose **best** result is right *and* clears the floor.
    pub top_1_kept: f32,
    /// The share with a right answer among the first [`REPORT_K`] results that **clear the floor** — what a person shown three
    /// would see.
    pub hit_3_kept: f32,
    /// The mean number of results per such query that clear the floor.
    pub returned: f32,
    /// Of everything those queries returned, the share that is right; `None` when nothing was returned.
    pub precision: Option<f32>,
    /// Of the queries that should find nothing, the share that return nothing; `None` when the set has none.
    pub silent: Option<f32>,
}

/// **The floor curve**: one [`FloorRow`] per floor, for choosing a floor with evidence rather than intuition.
///
/// A floor that is too low returns a library full of unrelated assets; one that is too high deletes the right answer. The curve
/// shows both costs at once — `top_1_kept` and `hit_3_kept` fall as the floor rises, `precision` and `silent` rise — so the
/// choice is a trade a person can read, and the numbers it was read from are the numbers that justify it.
pub fn floor_curve(cases: &[ScoredCase], floors: &[f32]) -> Vec<FloorRow> {
    let positives: Vec<&ScoredCase> = cases.iter().filter(|case| !case.is_negative()).collect();
    let negatives: Vec<&ScoredCase> = cases.iter().filter(|case| case.is_negative()).collect();
    floors
        .iter()
        .map(|&floor| {
            // `scored` is best first, so the results that clear the floor are a prefix of it. A NaN clears nothing.
            let clearing =
                |case: &ScoredCase| case.scored.iter().take_while(|(_, score)| *score >= floor).count();
            let (mut top_1, mut hit_3, mut returned, mut right) = (0usize, 0usize, 0usize, 0usize);
            for case in &positives {
                let n = clearing(case);
                returned += n;
                right += case.scored.iter().take(n).filter(|(hash, _)| case.right.contains(hash)).count();
                if case.scored.first().is_some_and(|(hash, _)| n > 0 && case.right.contains(hash)) {
                    top_1 += 1;
                }
                if case.scored.iter().take(n.min(REPORT_K)).any(|(hash, _)| case.right.contains(hash)) {
                    hit_3 += 1;
                }
            }
            let queries = positives.len().max(1) as f32;
            FloorRow {
                floor,
                top_1_kept: top_1 as f32 / queries,
                hit_3_kept: hit_3 as f32 / queries,
                returned: returned as f32 / queries,
                precision: (returned > 0).then(|| right as f32 / returned as f32),
                silent: (!negatives.is_empty()).then(|| {
                    negatives.iter().filter(|case| clearing(case) == 0).count() as f32
                        / negatives.len() as f32
                }),
            }
        })
        .collect()
}

/// A list of scores summarised by its quantiles, taken **by nearest rank** (`sorted[round((n − 1) · q)]`), so every figure is a
/// score that actually occurred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScoreBand {
    /// How many scores.
    pub n: usize,
    /// The smallest.
    pub min: f32,
    /// The 5th percentile.
    pub p05: f32,
    /// The 25th percentile.
    pub p25: f32,
    /// The median.
    pub median: f32,
    /// The 75th percentile.
    pub p75: f32,
    /// The 95th percentile.
    pub p95: f32,
    /// The largest.
    pub max: f32,
}

impl ScoreBand {
    /// The quantiles of `values`, ignoring any `NaN`; `None` for an empty list.
    pub fn of(mut values: Vec<f32>) -> Option<ScoreBand> {
        values.retain(|value| !value.is_nan());
        if values.is_empty() {
            return None;
        }
        values.sort_by(f32::total_cmp);
        let at = |q: f64| values[((values.len() - 1) as f64 * q).round() as usize];
        Some(ScoreBand {
            n: values.len(),
            min: values[0],
            p05: at(0.05),
            p25: at(0.25),
            median: at(0.5),
            p75: at(0.75),
            p95: at(0.95),
            max: values[values.len() - 1],
        })
    }
}

/// **How far apart the scores of right and wrong answers sit**: the three bands a floor is chosen between.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bands {
    /// The similarity of every right answer to the query it answers.
    pub right: ScoreBand,
    /// The similarity of every *wrong* asset to a query that has right answers.
    pub wrong: ScoreBand,
    /// The **best** score each query that should find nothing got: the highest a floor must clear to silence it. `None` when
    /// the set has no such query.
    pub negative_best: Option<ScoreBand>,
}

/// The three [`ScoreBand`]s of a set, or `None` when it has no positive query or no wrong answer to compare.
pub fn score_bands(cases: &[ScoredCase]) -> Option<Bands> {
    let (mut right, mut wrong, mut best) = (Vec::new(), Vec::new(), Vec::new());
    for case in cases {
        if case.is_negative() {
            best.extend(case.scored.first().map(|(_, score)| *score));
            continue;
        }
        for (hash, score) in &case.scored {
            if case.right.contains(hash) { right.push(*score) } else { wrong.push(*score) }
        }
    }
    Some(Bands {
        right: ScoreBand::of(right)?,
        wrong: ScoreBand::of(wrong)?,
        negative_best: ScoreBand::of(best),
    })
}
