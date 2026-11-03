//! **A description to a ranked answer, in one call.**
//!
//! The pieces existed and nothing joined them: [`Query`] validated a description, [`Vocabulary`] turned it into ids,
//! [`Model`] turned ids into a vector, and [`search`] ranked an index against a vector. A caller had to wire four things
//! in the right order and get three error types right, which is exactly the kind of seam that gets wired wrongly once
//! and then copied.
//!
//! [`Embedder::ask`] is that call. It owns no state and makes no decisions the pieces did not already make — it runs them in the
//! one order that is correct and reports every failure as one type.
//!
//! # Why the order is fixed, and why it is this one
//!
//! [`Query::new`] first, before any model work, because a description with no words in it should not reach a GPU.
//! Then the stamp check, because an index built by a different model cannot be ranked however good the embedding is.
//! Then tokenise, then embed, then rank. Each step's failure is a different sentence to a developer, and collapsing
//! them would lose the one that says what to do.

use super::index::{Index, ModelStamp};
use super::model::Model;
use super::query::{Query, QueryProblem};
use super::rank::Floor;
use super::search::{Answer, SearchProblem, search};
use super::tokenizer::{TokenProblem, Vocabulary};

/// Why a description could not be answered.
///
/// One type for the whole path, so a caller does not have to flatten three. Each variant is a distinct thing to do:
/// rephrase the description, load the right model, re-index, or fix the vocabulary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AskProblem {
    /// The description itself is unusable.
    Query(QueryProblem),
    /// The index was built by a different model than the one loaded.
    StampMismatch,
    /// The vocabulary cannot encode this description.
    Token(TokenProblem),
    /// The model could not be run.
    Model {
        /// The runtime's message, passed through rather than paraphrased.
        error: String,
    },
    /// The ranking refused, or the embedding did not fit the index.
    Search(SearchProblem),
}

impl core::fmt::Display for AskProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AskProblem::Query(q) => write!(f, "{q}"),
            AskProblem::StampMismatch => f.write_str(
                "this index was built by a different model than the one loaded. Re-index the library with this model.",
            ),
            AskProblem::Token(t) => write!(f, "{t}"),
            AskProblem::Model { error } => write!(f, "the model could not be run: {error}"),
            AskProblem::Search(s) => write!(f, "{s}"),
        }
    }
}

/// **A vocabulary and a model, together**, because a description needs both and they must agree.
///
/// Keeping them apart is how a caller ends up with a tokeniser for one model and weights for another: the ids are valid,
/// the vectors are valid, and the ranking is noise. This type holds the pair and offers [`Embedder::ask`].
pub struct Embedder<'a> {
    vocabulary: &'a Vocabulary,
    model: &'a mut Model,
}

impl<'a> Embedder<'a> {
    /// Pair a vocabulary with the model it belongs to.
    ///
    /// The identity of the pair is checked where it can be: [`Model::load`] reads the graph's own width from the file,
    /// and [`super::query::Query::tokenize`] refuses a description the vocabulary cannot encode. What cannot be checked
    /// here is whether the two came from the same *release* of the model — that is what the index's stamp is for, and
    /// [`Embedder::ask`] checks it before ranking.
    pub fn new(vocabulary: &'a Vocabulary, model: &'a mut Model) -> Embedder<'a> {
        Embedder { vocabulary, model }
    }

    /// **Encode a description**, checked against this vocabulary's token limit.
    pub fn encode(&self, query: &Query) -> Result<Vec<i64>, AskProblem> {
        let ids = query.tokenize(self.vocabulary).map_err(AskProblem::Query)?;
        Ok(ids.into_iter().map(i64::from).collect())
    }

    /// **Embed a description**, which is [`Embedder::encode`] then one model call.
    pub fn embed(&mut self, query: &Query) -> Result<Vec<f32>, AskProblem> {
        let ids = self.encode(query)?;
        let mask = self.vocabulary.attention_mask(&ids.iter().map(|&i| i as u32).collect::<Vec<_>>());
        self.model.embed_text(&ids, &mask).map_err(|e| AskProblem::Model { error: e.to_string() })
    }

    /// **Answer a description against an index**: the whole path, in the one order that is correct.
    ///
    /// `loaded` is the stamp of the model in use. It is passed rather than read from [`Model`] because a graph reports
    /// its *width*, not its release; the stamp is what a caller keeps beside the weights.
    pub fn ask(
        &mut self,
        index: &Index,
        loaded: &ModelStamp,
        description: &str,
        k: usize,
        floor: Floor,
    ) -> Result<Answer, AskProblem> {
        // 1. The description, before any model work: a query with no words in it should never reach a GPU.
        let query = Query::new(description).map_err(AskProblem::Query)?;
        // 2. The stamp, before the embedding: an index from another model cannot be ranked, whatever the vector is.
        if &index.stamp != loaded {
            return Err(AskProblem::StampMismatch);
        }
        // 3. Then the model, and 4. the ranking.
        let embedding = self.embed(&query)?;
        search(index, loaded, &query, &embedding, k, floor).map_err(AskProblem::Search)
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`AskProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for AskProblem {}
