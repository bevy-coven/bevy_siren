//! **One search, from a description to a ranked answer**, over the pieces this crate owns.
//!
//! Everything here is the crate's: a validated [`Query`], a stamped [`Index`], the [`top_k`] ranking and a [`Floor`].
//! The one thing it does not own is the embedding itself, and that is deliberate — it arrives as a slice, so the whole
//! pipeline is testable with no model, no device and no `ort`, and the model call is the single seam a caller supplies.
//!
//! # Why the seam is a slice and not a trait
//!
//! A trait would let a caller substitute a model, a cache or a mock. It would also let them substitute something that
//! is not CLAP, and then the index's stamp would be describing the wrong thing. A slice of `512` values that the index's
//! stamp already vouches for is narrower and harder to get wrong.

use super::index::{Index, ModelStamp};
use super::query::Query;
use super::rank::{Floor, Match, RankProblem, top_k};

/// Why a search could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchProblem {
    /// The index was built by a different model than the one that produced the embedding.
    StampMismatch,
    /// The embedding does not match the index's width.
    WrongWidth {
        /// The embedding's length.
        got: usize,
        /// The index's, from its stamp.
        expected: usize,
    },
    /// The ranking refused, for one of its own reasons.
    Rank(RankProblem),
}

impl core::fmt::Display for SearchProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SearchProblem::StampMismatch => {
                f.write_str("this index was built by a different model than the one that embedded the query")
            }
            SearchProblem::WrongWidth { got, expected } => {
                write!(f, "the query embedding has {got} values and this index holds {expected}")
            }
            SearchProblem::Rank(r) => write!(f, "{r}"),
        }
    }
}

/// **A ranked answer**: the results, and what was thrown away on the way.
///
/// `dropped` exists so a caller can say *"12 results, 4 below the similarity floor"*. A search that silently returns
/// three of sixteen is indistinguishable from a search that only found three, and that is the kind of silence this
/// crate refuses.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    /// The results, best first.
    pub matches: Vec<Match>,
    /// How many were below the floor and dropped.
    pub dropped: usize,
}

impl Answer {
    /// True when the floor removed everything the ranking found.
    pub fn filtered_out(&self) -> bool {
        self.matches.is_empty() && self.dropped > 0
    }

    /// A sentence a panel or a log can show without the caller composing one.
    pub fn describe(&self) -> String {
        if self.matches.is_empty() {
            return if self.dropped > 0 {
                format!("no result scored high enough; {} were below the floor", self.dropped)
            } else {
                "nothing in this library matched".to_string()
            };
        }
        if self.dropped == 0 {
            return format!("{} results", self.matches.len());
        }
        format!("{} results, {} below the floor", self.matches.len(), self.dropped)
    }
}

/// **Search an index by description**, given the embedding that description produced.
///
/// The three checks a caller cannot skip are here rather than at the call site: the stamp matches the model that made
/// the embedding, the width matches the index, and the query was validated before either. A search that skipped the
/// stamp check would rank across two vector spaces and look like a working feature doing a bad job.
pub fn search(
    index: &Index,
    loaded: &ModelStamp,
    query: &Query,
    embedding: &[f32],
    k: usize,
    floor: Floor,
) -> Result<Answer, SearchProblem> {
    if &index.stamp != loaded {
        return Err(SearchProblem::StampMismatch);
    }
    if embedding.len() != index.stamp.dimension {
        return Err(SearchProblem::WrongWidth { got: embedding.len(), expected: index.stamp.dimension });
    }
    // The query is already validated, but its token check is the caller's to make once a tokeniser exists; nothing
    // about the text reaches the index, so there is nothing here to check twice.
    let _ = query.text();

    let ranked = top_k(index, embedding, k).map_err(SearchProblem::Rank)?;
    let total = ranked.len();
    let matches = floor.apply(ranked);
    let dropped = total - matches.len();
    Ok(Answer { matches, dropped })
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`SearchProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for SearchProblem {}

/// **Everything a ranked search needs besides the index and the description**, so the call has one argument per idea.
///
/// Grouped into a type rather than passed as eight parameters: a call site with eight positional arguments is one whose
/// arguments get swapped silently, and `anchor` and `k` are both integers.
#[derive(Debug, Clone)]
pub struct Ranked<'a> {
    /// The embedding the description produced, on the index's own scale.
    pub embedding: &'a [f32],
    /// The asset the search is *relative to*: it supplies the acoustic features every candidate is judged against.
    pub anchor: u64,
    /// Acoustic features by content hash. An asset absent here is unjudged, never excluded.
    pub features: &'a std::collections::BTreeMap<u64, super::filter::Features>,
    /// How many results to return.
    pub k: usize,
    /// The similarity floor, applied to the model's own score before the ensemble reorders anything.
    pub floor: Floor,
    /// The project's weights.
    pub weights: &'a super::weights::Weights,
}

/// **One result with its reason attached** — `#40`'s per-result breakdown, beside the score it explains.
///
/// `similarity` is kept separate from `breakdown` because they answer different questions: the similarity is *what the
/// model saw*, on CLAP's own scale, and the breakdown is *why this ranked here* once the project's weights and the
/// acoustic rules had their say. A caller filtering on similarity must not accidentally filter on the weighted score.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored {
    /// The asset's content hash.
    pub hash: u64,
    /// The embedding similarity on the model's own scale, unchanged from [`Match::score`].
    pub similarity: f32,
    /// The ensemble's score: what the weighted signals sum to. **The ranking key** for [`search_ranked`].
    pub score: f32,
    /// Each signal's raw value, weight and contribution.
    pub breakdown: crate::model::Breakdown,
    /// Where the asset came from.
    pub provenance: super::provenance::Provenance,
}

impl Scored {
    /// **The sentence a panel or a log shows**: what carried the result, and what counted against it.
    pub fn explain(&self) -> String {
        self.breakdown.explain()
    }
}

/// **An answer whose results carry their own reasons.**
#[derive(Debug, Clone, PartialEq)]
pub struct RankedAnswer {
    /// The results, best first by ensemble score.
    pub matches: Vec<Scored>,
    /// How many were below the similarity floor and dropped.
    pub dropped: usize,
    /// How many the hard rules excluded — loudness or tempo beyond their tolerance.
    pub excluded: usize,
}

impl RankedAnswer {
    /// A sentence a panel or a log can show without the caller composing one.
    pub fn describe(&self) -> String {
        if self.matches.is_empty() {
            let mut s = String::from("nothing in this library matched");
            if self.excluded > 0 {
                s.push_str(&format!("; {} were excluded by the acoustic rules", self.excluded));
            }
            if self.dropped > 0 {
                s.push_str(&format!("; {} were below the similarity floor", self.dropped));
            }
            return s;
        }
        let mut s = format!("{} results", self.matches.len());
        if self.excluded > 0 {
            s.push_str(&format!(", {} excluded by the acoustic rules", self.excluded));
        }
        if self.dropped > 0 {
            s.push_str(&format!(", {} below the similarity floor", self.dropped));
        }
        s
    }
}

/// **Search, then score every candidate with the ensemble** — `#40`'s end-to-end path.
///
/// The order of operations is the design, so it is stated:
///
/// 1. `top_k` finds the `k` nearest by embedding similarity, on the model's own scale.
/// 2. `filter::evaluate` applies the **hard** rules; a candidate beyond the loudness or tempo tolerance is excluded and
///    never scored. That is what makes `#27`'s rules binding rather than advisory.
/// 3. `ensemble::breakdown` puts every surviving signal on the common `[0, 1]` scale and weights it.
/// 4. The results are **re-sorted by the ensemble score**, because a project's weights can reorder what the model
///    ranked — that is the whole point of tuning them.
///
/// `request.features` is keyed by content hash: an anchor the map does not hold is treated as having no acoustic features,
/// which yields an unjudged pair and a semantic-only score rather than a refusal. That is deliberate — an asset the
/// analysis could not measure should still be findable by description.
pub fn search_ranked(
    index: &Index,
    loaded: &ModelStamp,
    query: &Query,
    request: &Ranked,
) -> Result<RankedAnswer, SearchProblem> {
    if &index.stamp != loaded {
        return Err(SearchProblem::StampMismatch);
    }
    if request.embedding.len() != index.stamp.dimension {
        return Err(SearchProblem::WrongWidth {
            got: request.embedding.len(),
            expected: index.stamp.dimension,
        });
    }
    let _ = query.text();

    let k = request.k;
    let floor = request.floor;
    let weights = request.weights;
    let features = request.features;
    let anchor = request.anchor;
    let ranked = top_k(index, request.embedding, k).map_err(SearchProblem::Rank)?;
    let mut dropped = 0;
    let mut excluded = 0;
    let empty = super::filter::Features::default();
    let anchor_features = features.get(&anchor).copied().unwrap_or(empty);

    let mut matches = Vec::with_capacity(ranked.len());
    for m in ranked {
        if m.score < floor.min_score {
            dropped += 1;
            continue;
        }
        let candidate = features.get(&m.hash).copied().unwrap_or(empty);
        let judgement = super::filter::evaluate(&anchor_features, &candidate, weights);
        if !judgement.admitted() {
            excluded += 1;
            continue;
        }
        let breakdown =
            super::ensemble::breakdown(m.score, &candidate, &anchor_features, &judgement, weights);
        matches.push(Scored {
            hash: m.hash,
            similarity: m.score,
            score: breakdown.score(),
            breakdown,
            provenance: m.provenance,
        });
    }

    // **Descending ensemble score, then ascending hash.** A total order, so two runs cannot disagree.
    matches.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(core::cmp::Ordering::Equal).then_with(|| a.hash.cmp(&b.hash))
    });
    matches.truncate(k);
    Ok(RankedAnswer { matches, dropped, excluded })
}
