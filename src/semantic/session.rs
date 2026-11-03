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

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`LoadProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for LoadProblem {}

/// **A batch run's outcome**, so a shrink or a retry is visible rather than inferred — `#18`.
#[derive(Debug, Clone, PartialEq)]
pub struct Batched {
    /// The vectors, in the caller's order. **The order is the contract**: a batch that reordered would silently pair
    /// every vector with the wrong asset.
    pub vectors: Vec<Vec<f32>>,
    /// The batch sizes actually used, in order. A caller comparing this to the configured maximum is how `#18`'s
    /// pitfall — *"if the batch shrinks on every call without being reported, `P-01` will measure a number that cannot
    /// be reproduced"* — is avoided.
    pub batches: Vec<usize>,
    /// How many times a batch was refused and retried smaller.
    pub shrinks: usize,
}

impl Batched {
    /// The largest batch this run actually achieved, which is the number a throughput figure must be quoted against.
    pub fn largest_batch(&self) -> usize {
        self.batches.iter().copied().max().unwrap_or(0)
    }

    /// A sentence a log or a panel shows, naming any shrink rather than hiding it.
    pub fn describe(&self) -> String {
        if self.shrinks == 0 {
            return format!(
                "{} vectors in {} batches, largest {}",
                self.vectors.len(),
                self.batches.len(),
                self.largest_batch()
            );
        }
        format!(
            "{} vectors in {} batches, largest {}, after {} shrink(s) — the ceiling was not reachable",
            self.vectors.len(),
            self.batches.len(),
            self.largest_batch(),
            self.shrinks
        )
    }
}

/// Why a batch run could not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchProblem {
    /// The configured maximum is zero, so nothing can be embedded.
    ZeroBatch,
    /// The embedder refused even a single item, so shrinking cannot help.
    SingleItemRefused {
        /// The refusal, in the embedder's own words.
        why: String,
    },
    /// The per-item size was zero, so no capacity can be computed.
    ZeroItemSize,
}

impl core::fmt::Display for BatchProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BatchProblem::ZeroBatch => {
                f.write_str("the batch maximum is zero: a batch of nothing embeds nothing")
            }
            BatchProblem::SingleItemRefused { why } => {
                write!(f, "the embedder refused a single item, so a smaller batch cannot help: {why}")
            }
            BatchProblem::ZeroItemSize => {
                f.write_str("the per-item size is zero, so no batch capacity exists")
            }
        }
    }
}

impl std::error::Error for BatchProblem {}

/// **Embed a list of buffers in batches, shrinking on refusal** — `#18`.
///
/// `embed` is the caller's one call into the runtime; it takes a slice of buffers and returns their vectors, or a
/// refusal when the batch is too large. The two behaviours `#18` asks for are the whole of this function:
///
/// - **the maximum is a configured ceiling**, and the actual sizes used are recorded in [`Batched::batches`];
/// - **a refusal shrinks the batch and continues**, reporting it. Halving is the strategy rather than decrementing by
///   one, because an out-of-memory refusal is a statement about the size of a batch rather than about a single item, and
///   halving reaches a workable size in `log2` attempts instead of a linear number.
///
/// **Refusing at size one is a failure, not a shrink.** If the embedder will not accept one item, the graph is wrong or
/// the item is too large for the device, and retrying smaller is not possible — so it is reported rather than silently
/// returning fewer vectors than the caller asked for. That is the difference between a shrink and a failure, and it is
/// the one thing this function must not blur.
pub fn embed_batched(
    buffers: &[Vec<f32>],
    maximum: usize,
    mut embed: impl FnMut(&[Vec<f32>]) -> Result<Vec<Vec<f32>>, String>,
) -> Result<Batched, BatchProblem> {
    if maximum == 0 {
        return Err(BatchProblem::ZeroBatch);
    }
    let mut vectors = Vec::with_capacity(buffers.len());
    let mut batches = Vec::new();
    let mut shrinks = 0;
    let mut current = maximum.min(buffers.len().max(1));

    let mut at = 0usize;
    while at < buffers.len() {
        let take = current.min(buffers.len() - at);
        match embed(&buffers[at..at + take]) {
            Ok(produced) => {
                // **A short answer is a failure.** Accepting fewer vectors than buffers would pair them with the wrong
                // assets from that point on, which is the kind of silent corruption this crate refuses everywhere.
                if produced.len() != take {
                    return Err(BatchProblem::SingleItemRefused {
                        why: format!("the embedder returned {} vectors for {take} buffers", produced.len()),
                    });
                }
                vectors.extend(produced);
                batches.push(take);
                at += take;
                // Recover toward the ceiling after a successful batch, so one early refusal does not cap the whole run.
                if current < maximum {
                    current = (current * 2).min(maximum);
                }
            }
            Err(why) => {
                if take == 1 {
                    return Err(BatchProblem::SingleItemRefused { why });
                }
                shrinks += 1;
                current = (take / 2).max(1);
            }
        }
    }
    Ok(Batched { vectors, batches, shrinks })
}
