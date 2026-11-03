//! **A text query on its way to the model**: the checks that are this crate's, and nothing that is the tokeniser's.
//!
//! The crate cannot tokenise without a dependency it does not have. What it *can* do is hold the contract the
//! tokeniser must meet, so a caller that supplies one — or a later implementation inside the crate — has a single
//! place that says what a valid query is, and what a bad one is called.
//!
//! Measured ground truth for the numbers here is `runs/2026-11-03-63/research.md` S2: a 1-word query produced 4
//! tokens and a 12-word one 15, both through the same exported graph.

use super::MAX_TEXT_TOKENS;
use super::tokenizer::{TokenProblem, Vocabulary};

/// Why a description cannot be sent to the model.
///
/// Every variant names what was found, because the message is read by a person who typed the description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryProblem {
    /// The description is empty or only whitespace.
    Empty,
    /// The description would need more tokens than the model accepts.
    TooLong {
        /// Tokens the description needs, as counted by the caller's tokeniser.
        need: usize,
        /// What the model accepts: [`MAX_TEXT_TOKENS`].
        limit: usize,
    },
    /// A description that cannot be a query because it carries no words the model could attach meaning to.
    NoWords,
}

impl core::fmt::Display for QueryProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QueryProblem::Empty => f.write_str("the description is empty"),
            QueryProblem::TooLong { need, limit } => write!(
                f,
                "the description needs {need} tokens and the model accepts {limit}. Shorten it, or split it into \
                 two searches."
            ),
            QueryProblem::NoWords => f.write_str("the description has no words in it"),
        }
    }
}

/// **A description that has passed this crate's checks.** The only way to get one is [`Query::new`], so a caller
/// cannot hand the model something that was never validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Query {
    text: String,
}

impl Query {
    /// **Validate a description and keep it**, without tokenising.
    ///
    /// Trimming is deliberate: a description padded with spaces produces the same tokens, so the stored form is the
    /// one a person would read back in a log.
    pub fn new(description: &str) -> Result<Query, QueryProblem> {
        let text = description.trim();
        if text.is_empty() {
            return Err(QueryProblem::Empty);
        }
        // A description made only of punctuation or digits has nothing the text tower was trained to attach to. It is
        // refused rather than sent, because a query that returns an arbitrary ranking looks like a working feature.
        if !text.chars().any(|c| c.is_alphabetic()) {
            return Err(QueryProblem::NoWords);
        }
        Ok(Query { text: text.to_string() })
    }

    /// **Check the token count the tokeniser reported**, once there is a tokeniser to ask.
    ///
    /// Kept separate from [`Query::new`] so the crate validates what it can without a dependency, and validates the
    /// rest at the one place the count is actually known.
    pub fn check_tokens(&self, tokens: usize) -> Result<(), QueryProblem> {
        if tokens > MAX_TEXT_TOKENS {
            return Err(QueryProblem::TooLong { need: tokens, limit: MAX_TEXT_TOKENS });
        }
        Ok(())
    }

    /// The description as it will be sent: trimmed.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// **Tokenise the description with CLAP's own vocabulary**, checking the limit at the one place the count is known.
    ///
    /// This is the seam that was missing: [`Query`] validated the text, [`crate::semantic::model::Model::embed_text`]
    /// takes ids, and nothing turned one into the other. A query longer than [`MAX_TEXT_TOKENS`] is refused with both
    /// counts rather than silently truncated, because a truncated description is a different description.
    pub fn tokenize(&self, vocabulary: &Vocabulary) -> Result<Vec<u32>, QueryProblem> {
        let ids = vocabulary.encode_for_model(&self.text).map_err(|e| match e {
            TokenProblem::UnknownToken { .. } | TokenProblem::Vocabulary { .. } => QueryProblem::NoWords,
        })?;
        // The specials are not part of the model's text budget, so they are counted out.
        let body = ids.len().saturating_sub(2);
        self.check_tokens(body)?;
        Ok(ids)
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`QueryProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for QueryProblem {}
