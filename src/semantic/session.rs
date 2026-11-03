//! **A loaded model, and the budget that decided it could be loaded.**
//!
//! The crate's GPU rule is the voice rule: a resource that refuses politely, with the numbers, rather than failing
//! inside a library. [`crate::semantic::budget`] decides whether a model fits; this module is what happens after the
//! answer is yes — and it keeps the numbers so a caller can report them.
//!
//! # What it does not do
//!
//! It does not choose a model for you, and it does not download one. `S-05`'s licence audit decides which weights may
//! ship, and #17's notes record that a pinned download is a separate issue. A `Session` is given a path that already
//! exists and a budget that already permitted it.
//!
//! # The measured facts it is built around
//!
//! From `runs/2026-11-03-63/evidence/ort-probe.txt`: `ort` reports the graph's dynamic axes, the text tower accepts
//! `[-1, -1]` for batch and tokens, and the output is already unit-norm. From the audio spike: the audio tower's batch
//! axis is **broken** above 1, so a session must not assume one graph accepts any batch.

use super::budget::{Budget, BudgetProblem, ModelCost};

/// What a session says about the graph it loaded, read from the runtime rather than assumed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    /// The names of the graph's inputs, in the order the runtime reports them.
    pub inputs: Vec<String>,
    /// The names of its outputs.
    pub outputs: Vec<String>,
    /// The dimension of the value it returns, from the output shape.
    pub dimension: usize,
    /// Whether the batch axis is symbolic, which is the difference between a graph that takes 8 items and one that
    /// must be exported per batch size. Measured **false** for CLAP's audio tower and **true** for its text tower.
    pub dynamic_batch: bool,
}

/// Why a model could not be loaded.
#[derive(Debug, Clone, PartialEq)]
pub enum LoadProblem {
    /// The budget refused before the runtime was asked.
    Budget(BudgetProblem),
    /// The model's file could not be read.
    Unreadable {
        /// The path, so the message points at a file.
        path: String,
        /// The runtime's or the filesystem's message.
        error: String,
    },
    /// The graph's output is not the width the index expects, so its vectors could never be compared.
    DimensionMismatch {
        /// What the graph returned.
        graph: usize,
        /// What the index holds.
        expected: usize,
    },
}

impl core::fmt::Display for LoadProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadProblem::Budget(b) => write!(f, "{b}"),
            LoadProblem::Unreadable { path, error } => write!(f, "cannot load `{path}`: {error}"),
            LoadProblem::DimensionMismatch { graph, expected } => write!(
                f,
                "this graph returns {graph}-element vectors and the index holds {expected}. Re-index the library \
                 with this model."
            ),
        }
    }
}

/// **A model that is loaded and may be used.** Its existence is the proof the budget permitted it.
#[derive(Debug)]
pub struct Session {
    loaded: Loaded,
    cost: ModelCost,
}

impl Session {
    /// **Decide, then report.** The budget is checked first, so a refusal happens before any device work.
    ///
    /// `loaded` is what the runtime reported about the graph, which the caller reads. This is the seam that keeps the
    /// crate testable with no `ort` in the build: the budget decision and the shape rules are here, and reading a
    /// graph is the caller's one call into the runtime.
    pub fn admit(
        budget: &Budget,
        cost: ModelCost,
        loaded: Loaded,
        expected_dimension: usize,
    ) -> Result<Session, LoadProblem> {
        budget.fits(cost).map_err(LoadProblem::Budget)?;
        if loaded.dimension != expected_dimension {
            return Err(LoadProblem::DimensionMismatch {
                graph: loaded.dimension,
                expected: expected_dimension,
            });
        }
        Ok(Session { loaded, cost })
    }

    /// What the runtime said about the graph.
    pub fn loaded(&self) -> &Loaded {
        &self.loaded
    }

    /// What the budget committed for it.
    pub fn cost(&self) -> ModelCost {
        self.cost
    }

    /// **The batch sizes this graph can be handed**, given the budget it was admitted against and the measured
    /// per-item activation cost.
    ///
    /// The budget is a parameter rather than a field because capacity is a function of *what is free now*, which
    /// changes as other things commit memory; a session that remembered the number it was admitted with would report a
    /// stale capacity.
    ///
    /// A graph whose batch axis is symbolic takes the budget's capacity; one whose axis is fixed takes **1**, because
    /// CLAP's audio tower bakes its batch into a `Reshape` and fails at any other size. Returning the capacity for a
    /// fixed-axis graph would be the optimistic answer, and it is the one that fails at run time.
    pub fn batch_sizes(&self, budget: &Budget, per_item_bytes: u64) -> Vec<u64> {
        if !self.loaded.dynamic_batch {
            return vec![1];
        }
        let capacity = budget.batch_capacity(self.cost, per_item_bytes);
        (1..=capacity.max(1)).collect()
    }
}
