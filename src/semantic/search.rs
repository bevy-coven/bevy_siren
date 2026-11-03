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
