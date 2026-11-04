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
use super::provenance::Provenance;

/// **The query latency budget**, measured rather than hoped for — `#47`.
///
/// A panel that stalls on a keystroke is a panel nobody uses, so the crate's own per-query work has a **ceiling stated
/// as a number**, and this constant is it. `examples/querybench` is what produces the figures:
///
/// | assets | index scan p50 | p99 | full path p50 | p99 |
/// |---|---|---|---|---|
/// | 1 000 | **0.41 ms** | **0.91 ms** | 0.39 ms | 0.87 ms |
/// | 10 000 | **4.30 ms** | **4.55 ms** | 4.26 ms | 4.55 ms |
///
/// 120 queries per size, `k = 3`, 512 dimensions, a released build on this Mac. "Full path" adds the stamp check, the
/// hard acoustic rules, the ensemble breakdown and the re-sort, so the two rows say the **ranking dominates and the rest
/// is noise** — the whole retrieval path costs what the scan costs.
///
/// # Why 10 ms, when the measurement is 4.55
///
/// Twice the worst case at 10 000 assets, which is the largest library the crate's own examples model. The headroom is
/// for the machine rather than the algorithm: a library twice this size would land at ~9 ms, and a slower host or a
/// busier one could double the figure without the code changing. A budget equal to the measurement would be a tripwire
/// that fires on a laptop.
///
/// # What happens when it is exceeded
///
/// **Nothing automatic**, and that is the decision rather than an omission: silently narrowing `k` would return a
/// different answer to the one asked for, which is the class of bug this crate refuses everywhere else. A caller that
/// measures over budget has the two facts it needs — [`PER_QUERY_BUDGET_MS`] and its own timing — and can decide for
/// itself. What this constant forbids is shipping a regression without noticing.
///
/// # The half this does not cover
///
/// The **text encode** is not in these numbers: it needs the ONNX graph and `ort`, which the bench does not link. ADR
/// 0002 measured the text tower at **5.28 ms per query** on an M4 CPU, so the encode is the larger half of a cold query
/// and a panel that feels slow is waiting on the model, not on this crate.
pub const PER_QUERY_BUDGET_MS: f64 = 10.0;

/// The asset count the budget is stated for.
pub const BUDGET_ASSETS: usize = 10_000;

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

/// One result: which asset, how similar, and **where it came from**.
///
/// `provenance` is carried on every result rather than left to a second lookup, because the alternative lets a
/// consumer forget — and a result that omits its licence presents an unlicensed asset as if it had no obligation.
/// That costs this type its `Copy`: two of the three provenance fields are `String`s. A caller that iterated by value
/// now iterates by reference.
#[derive(Debug, Clone, PartialEq)]
pub struct Match {
    /// The asset's content hash.
    pub hash: u64,
    /// Cosine similarity in `[-1, 1]`. The model's embeddings are normalised, so in practice this is `[0, 1]`.
    pub score: f32,
    /// Where the asset came from, or the explicit unknown when nothing was recorded.
    pub provenance: Provenance,
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
            Some(score) => scored.push(Match {
                hash: *hash,
                score,
                // An asset with no record carries `Provenance::unknown()` — a value, not an absence, so a caller
                // printing a result always has something to print.
                provenance: index.provenance.get(hash).cloned().unwrap_or_default(),
            }),
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

/// The cosine a **correct** text-to-audio match scored in this crate's **first** measurement: six synthetic clips and five
/// descriptions, where the right clip scored between **0.126** and **0.429** (`runs/2026-11-03-63/research.md` S3).
///
/// **Superseded.** On the golden set (`#35`: 2 000 real recordings, 50 descriptions) the right answers' median is **0.468** and
/// their 5th percentile **0.273** — the best of the six synthetic matches (0.429) is below the real median. Synthetic tones
/// understate what a real match scores, which is the lesson in `docs/guide/golden-set.md`. The constant stays because callers and
/// tests use it as a number to clear, not because it is the measurement.
pub const MEASURED_MATCH_FLOOR: f32 = 0.126;

/// The cosine a **wrong** match scored in the same first measurement: **−0.104** to **+0.150**.
///
/// **Superseded** with [`MEASURED_MATCH_FLOOR`]. The six-clip bands overlapped; the real ones overlap only in their tails — the right
/// answers' 5th percentile is 0.273 and the wrong answers' 95th is 0.277 — and an absolute floor on a query's *best* result tells a
/// description the library can answer from one it cannot with an AUC of **0.963** (**0.885** when the same sounds are described in
/// a stranger's words; `docs/guide/golden-set.md`).
pub const MEASURED_MISS_CEILING: f32 = 0.150;

/// **A similarity floor**, so a result set can be empty rather than full of unrelated assets.
///
/// A cosine near zero between two tracks means they are close to orthogonal; returning it as a "match" wastes a developer's
/// attention. The floor is a named field because the number that works depends on the model and the library.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
    /// Results below this cosine are dropped.
    ///
    /// **Design default: `0.0`**, which keeps everything — **a measurement, not timidity, now on two libraries.** The golden set's first
    /// library (2 000 recordings) says a floor of 0.35 costs nothing there; its second (303 game sounds from Kenney's CC0 packs) says the
    /// same floor returns nothing at all for 13 of 29 descriptions and loses a fifth to a half of the hits (top-1 31 → 21 %, hit@3 52 →
    /// 28 % in the maintainer's wording), because a game's right answers score a median of 0.191 where the first library's score 0.468
    /// and the strongest top result of a description that should find
    /// *nothing* (0.490) is above the best result of 27 of the 29 that should find something. **The largest floor that costs neither
    /// library an answer, in either of two wordings, is 0.20, and it silences no description that should find nothing** — so no
    /// positive default does what a floor is for, and a floor *relative to the query* (`eval::centred`) is worse on the second library
    /// still. A default every consumer inherits is the owner's to change; `docs/loop/needs-decision/default-floor.md` has the tables, the
    /// options and the recommendation: keep `0.0`.
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
