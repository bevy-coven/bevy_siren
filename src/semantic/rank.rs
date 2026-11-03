//! **Ranking: cosine similarity, and a top-k that is honest about ties and refusals.**
//!
//! A query embedding against every vector in an index. The arithmetic is four lines; the decisions are not, and they
//! are why this module exists separately from any model call:
//!
//! - **CLAP normalises its embeddings** before returning them (measured in `runs/2026-11-03-62/research.md`), so a
//!   cosine is a dot product of unit vectors. This module does not assume that: it computes a real cosine, so an index
//!   built by a future model that does not normalise still ranks correctly rather than silently returning a dot
//!   product that is not a similarity.
//! - **A zero vector has no cosine.** Its norm is zero, so the division is undefined; it is reported as
//!   [`RankProblem::DegenerateVector`] rather than as a score of `0.0`, which would look like "not similar" instead of
//!   "this vector cannot be compared".
//! - **`k = 0` is a refusal**, not an empty answer: an agent that asked for nothing made a mistake, and an empty list
//!   would hide it.
//! - **Ordering is total and reproducible.** Equal scores break by content hash, so two runs over the same index
//!   return the same order field for field — the crate's determinism rule, applied to a ranking.

use std::cmp::Ordering;

use super::index::Index;

/// Why a query could not be ranked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RankProblem {
    /// The query asked for no results.
    ZeroK,
    /// A vector in the index, or the query itself, is all zeros, so a cosine is undefined.
    DegenerateVector {
        /// The content hash of the offending vector, or `None` when it was the query.
        hash: Option<u64>,
    },
    /// The query vector is not the index's dimension.
    DimensionMismatch {
        /// The query's length.
        query: usize,
        /// The index's, from its stamp.
        index: usize,
    },
}

impl core::fmt::Display for RankProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            RankProblem::ZeroK => f.write_str("a query for zero results is not a query"),
            RankProblem::DegenerateVector { hash } => match hash {
                Some(h) => write!(f, "the vector for asset {h:016x} is all zeros, so it has no similarity"),
                None => f.write_str("the query vector is all zeros, so it has no similarity to anything"),
            },
            RankProblem::DimensionMismatch { query, index } => {
                write!(f, "the query has {query} values and this index holds {index}")
            }
        }
    }
}

/// One result: which asset, and how similar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    /// The asset's content hash.
    pub hash: u64,
    /// Cosine similarity in `[-1, 1]`. The model's embeddings are normalised, so in practice this is `[0, 1]`.
    pub score: f32,
}

/// **Cosine similarity** of two equal-length slices, or `None` when either has no direction.
///
/// Returns `None` for a zero vector rather than `0.0`: those are different facts and a caller must be able to tell
/// them apart.
pub fn cosine(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let mut dot = 0.0f64;
    let mut na = 0.0f64;
    let mut nb = 0.0f64;
    for (&x, &y) in a.iter().zip(b) {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        dot += f64::from(x) * f64::from(y);
        na += f64::from(x) * f64::from(x);
        nb += f64::from(y) * f64::from(y);
    }
    if na <= 0.0 || nb <= 0.0 {
        return None;
    }
    Some((dot / (na.sqrt() * nb.sqrt())) as f32)
}

/// **The `k` most similar assets to `query`**, best first.
///
/// Ties are broken by content hash so the order is total, and a vector that cannot be compared is refused rather than
/// scored. A query whose length disagrees with the index is refused before any comparison.
pub fn top_k(index: &Index, query: &[f32], k: usize) -> Result<Vec<Match>, RankProblem> {
    if k == 0 {
        return Err(RankProblem::ZeroK);
    }
    let dimension = index.stamp.dimension;
    if query.len() != dimension {
        return Err(RankProblem::DimensionMismatch { query: query.len(), index: dimension });
    }
    match cosine(query, query) {
        Some(_) => {}
        None => return Err(RankProblem::DegenerateVector { hash: None }),
    }

    let mut scored: Vec<Match> = Vec::with_capacity(index.records.len());
    for (hash, vector) in &index.records {
        match cosine(query, vector) {
            Some(score) => scored.push(Match { hash: *hash, score }),
            None => return Err(RankProblem::DegenerateVector { hash: Some(*hash) }),
        }
    }

    scored.sort_by(|a, b| {
        // Descending score, then ascending hash: a total order, so two runs cannot differ.
        b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal).then_with(|| a.hash.cmp(&b.hash))
    });
    scored.truncate(k);
    Ok(scored)
}

/// The cosine a **correct** text-to-audio match scored in this crate's own measurement.
///
/// Measured on six synthetic clips and five descriptions: the right clip scored between **0.126** and **0.429**
/// (`runs/2026-11-03-63/research.md` S3).
pub const MEASURED_MATCH_FLOOR: f32 = 0.126;

/// The cosine a **wrong** match scored in the same measurement: **−0.104** to **+0.150**.
///
/// **The bands overlap with [`MEASURED_MATCH_FLOOR`]**, which is the single most important fact this module's defaults
/// are built around. A floor of 0.5 — the number intuition suggests — would reject every correct match.
pub const MEASURED_MISS_CEILING: f32 = 0.150;

/// **A similarity floor**, so a result set can be empty rather than full of unrelated assets.
///
/// A cosine near zero between two tracks means they are close to orthogonal; returning it as a "match" wastes a
/// developer's attention. The floor is a named field because the number that works depends on the model and the
/// library, and `V-01`'s labelled set is what replaces it with a measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
    /// Results below this cosine are dropped.
    ///
    /// **Design default: `0.0`**, which keeps everything.
    ///
    /// The measurement says the correct and incorrect bands **overlap** ([`MEASURED_MATCH_FLOOR`] is *below*
    /// [`MEASURED_MISS_CEILING`]), so no single threshold separates them: a floor that removes the wrong matches also
    /// removes right ones. A long list with scores a caller can weigh beats a short list that has silently dropped the
    /// answer. When `V-01`'s labelled set exists, this default becomes the threshold that maximises recall at an
    /// acceptable precision, and the number will be recorded here with the measurement that chose it.
    pub min_score: f32,
}

impl Default for Floor {
    fn default() -> Self {
        Floor { min_score: 0.0 }
    }
}

impl Floor {
    /// Keep only results at or above the floor.
    pub fn apply(&self, results: Vec<Match>) -> Vec<Match> {
        results.into_iter().filter(|m| m.score >= self.min_score).collect()
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`RankProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for RankProblem {}
