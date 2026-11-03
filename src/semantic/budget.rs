//! **A GPU budget that refuses politely**, the way the voice budget does.
//!
//! The crate already has this idea for voices: a bus with a cap refuses with [`crate::play::Refusal::VoiceLimit`]
//! rather than dropping the newest sound. A GPU is the same kind of resource and gets the same treatment — a request
//! that does not fit is **refused with the numbers**, never attempted and failed inside a library.
//!
//! # Why the arithmetic lives here
//!
//! It is the half of model loading that needs no device: what fits, what must be evicted, what a batch of `n` costs.
//! `src/model.rs`'s rule applies — the part that can be tested with no hardware is the part that decides the numbers,
//! and a defect there would otherwise surface as an out-of-memory three layers down.
//!
//! # The numbers this uses, and where they come from
//!
//! The audio tower is **27,549,128 parameters** and exports to **116.5 MB** as fp32 ONNX, both measured in
//! `runs/2026-11-03-62/research.md`. The *context* cost is not measured on a GPU yet, so it is a named field with a
//! **design default** and an explicit note that `P-01` must replace it.

/// What one loaded model costs, so a budget can decide before it asks a driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCost {
    /// The weights on the device, in bytes.
    pub weights_bytes: u64,
    /// The context and workspace the runtime allocates beside the weights, in bytes. **Design default**: a CUDA
    /// context is several hundred MB before any weights, and [#17](https://github.com/bevy-coven/bevy_siren/issues/17)
    /// records that this must be replaced by a measurement on the target device.
    pub overhead_bytes: u64,
}

impl ModelCost {
    /// The CLAP audio tower as measured: 116.5 MB of weights.
    ///
    /// `116_500_000` is the spike's `onnx_mb` figure. The overhead is the **design default** above, not a measurement.
    pub fn measured_clap_audio() -> Self {
        ModelCost { weights_bytes: 116_500_000, overhead_bytes: DEFAULT_OVERHEAD_BYTES }
    }

    /// Everything the model occupies: weights plus overhead.
    pub fn total_bytes(&self) -> u64 {
        self.weights_bytes.saturating_add(self.overhead_bytes)
    }
}

/// **Design default.** The runtime's own cost beside the weights.
///
/// 512 MiB. A CUDA context costs hundreds of megabytes before a single weight is uploaded, and a budget that ignores
/// it passes on the first model and fails on the second — which is the failure this constant exists to make visible.
/// `P-01` measures the real figure on the RTX 3090 and replaces this.
pub const DEFAULT_OVERHEAD_BYTES: u64 = 512 * 1024 * 1024;

/// What a device has, and what is already committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Budget {
    /// Total device memory reported by the driver.
    pub total_bytes: u64,
    /// Already committed by anything else — the editor's renderer, another process.
    pub used_bytes: u64,
    /// This crate's own ceiling, below the device's. `None` means "use whatever the device reports free".
    pub ceiling_bytes: Option<u64>,
}

/// Why a load was refused. Every variant carries the numbers, because the message is read by someone deciding whether
/// to close the editor, lower a setting, or buy a bigger card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetProblem {
    /// The request does not fit in what is available.
    DoesNotFit {
        /// What the load needs.
        need_bytes: u64,
        /// What is available now.
        available_bytes: u64,
    },
    /// This crate's own ceiling would be exceeded even though the device has room.
    OverCeiling {
        /// What would be committed after the load.
        would_commit_bytes: u64,
        /// The ceiling that forbids it.
        ceiling_bytes: u64,
    },
    /// The device reports less total than it reports used, which means a driver answered something impossible.
    Inconsistent {
        /// Total reported.
        total_bytes: u64,
        /// Used reported.
        used_bytes: u64,
    },
}

impl core::fmt::Display for BudgetProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mb = |b: u64| format!("{:.0} MB", b as f64 / 1_048_576.0);
        match self {
            BudgetProblem::DoesNotFit { need_bytes, available_bytes } => write!(
                f,
                "this needs {} and {} is free. Close what is using the GPU, or load fewer models.",
                mb(*need_bytes),
                mb(*available_bytes)
            ),
            BudgetProblem::OverCeiling { would_commit_bytes, ceiling_bytes } => write!(
                f,
                "this would commit {} of a {} ceiling this project set. Raise the ceiling, or load fewer models.",
                mb(*would_commit_bytes),
                mb(*ceiling_bytes)
            ),
            BudgetProblem::Inconsistent { total_bytes, used_bytes } => write!(
                f,
                "the device reports {} used of {} total, which cannot be right",
                mb(*used_bytes),
                mb(*total_bytes)
            ),
        }
    }
}

impl Budget {
    /// **What the device has free**, ignoring this crate's ceiling.
    ///
    /// The ceiling is deliberately *not* folded in here. It is the project's own setting, and a project that set a
    /// ceiling should be told about the ceiling rather than about the card — two different numbers in a message are
    /// worse than one, because they send the reader to fix the wrong thing.
    pub fn device_free_bytes(&self) -> Result<u64, BudgetProblem> {
        if self.used_bytes > self.total_bytes {
            return Err(BudgetProblem::Inconsistent {
                total_bytes: self.total_bytes,
                used_bytes: self.used_bytes,
            });
        }
        Ok(self.total_bytes - self.used_bytes)
    }

    /// **What this crate may use**: the device's free memory, or the ceiling's remaining headroom, whichever is smaller.
    ///
    /// A ceiling below what is already committed leaves zero, not a wrapped number.
    pub fn available_bytes(&self) -> Result<u64, BudgetProblem> {
        let free = self.device_free_bytes()?;
        Ok(match self.ceiling_bytes {
            Some(c) => free.min(c.saturating_sub(self.used_bytes.min(c))),
            None => free,
        })
    }

    /// **Whether `cost` can be loaded now**, and why not when it cannot.
    ///
    /// The ceiling is checked separately from the device, so a project that set one gets a message about *its* setting
    /// rather than about the card.
    pub fn fits(&self, cost: ModelCost) -> Result<(), BudgetProblem> {
        // The device first: a request the hardware cannot hold is a device problem however low the ceiling is.
        let free = self.device_free_bytes()?;
        let need = cost.total_bytes();
        if need > free {
            return Err(BudgetProblem::DoesNotFit { need_bytes: need, available_bytes: free });
        }
        // Then the project's own ceiling, reported as the ceiling it is.
        if let Some(ceiling) = self.ceiling_bytes {
            let would_commit = self.used_bytes.saturating_add(need);
            if would_commit > ceiling {
                return Err(BudgetProblem::OverCeiling {
                    would_commit_bytes: would_commit,
                    ceiling_bytes: ceiling,
                });
            }
        }
        Ok(())
    }

    /// **The largest batch whose activations fit** in what is left after `cost`'s weights and overhead.
    ///
    /// `per_item_bytes` is the activation cost of one item, which the caller measures. The answer is at least 1:
    /// a batch of zero is not a smaller batch, it is a refusal, and that is [`Budget::fits`]'s decision rather than
    /// this one's.
    pub fn batch_capacity(&self, cost: ModelCost, per_item_bytes: u64) -> u64 {
        if per_item_bytes == 0 {
            return 1;
        }
        let after = self.available_bytes().unwrap_or(0).saturating_sub(cost.total_bytes());
        (after / per_item_bytes).max(1)
    }
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`BudgetProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for BudgetProblem {}
