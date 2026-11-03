//! **The model, as the crate calls it.** The one place `ort` appears in this crate.
//!
//! Everything else in [`crate::semantic`] takes an embedding as a slice, so it is testable with no device, no weights
//! and no `ort`. This module is the seam: it loads a graph, reports what the graph says about itself, and runs it.
//!
//! # What it will not do
//!
//! It does not download weights. `S-05`'s licence audit decides which weights may ship and `#17` records that a pinned
//! download is its own issue, so [`Model::load`] takes a path that already exists and says so when it does not.
//!
//! It does not choose a batch size. Measured in `runs/2026-11-03-62/research.md`: CLAP's **audio** tower bakes its
//! batch axis into a `Reshape` and fails above batch 1, while its **text** tower reports `[-1, -1]`. A caller that
//! assumed one accepted any batch would be wrong half the time, so [`Model::loaded`] carries what the runtime said and
//! [`crate::semantic::session::Session::batch_sizes`] is the only thing that decides.

use std::sync::OnceLock;

use ort::session::Session as OrtSession;
use ort::value::Tensor;

use super::MODEL_MEL_FRAMES;
use super::session::{LoadProblem, Loaded};

/// **One ONNX Runtime environment per process**, created once.
///
/// `ort` refuses a second: *"only one environment is allowed per process"*. That is not a detail a caller should have
/// to know, and a `Model::load` that created one each time would work in a test that loads one model and **fail in
/// every real application**, which loads at least a text tower and an audio tower.
///
/// Found by the tests running together rather than one at a time — `ids_and_mask_of_different_lengths…` passed alone
/// and failed in the suite, which is exactly the shape of a process-wide constraint.
static ENVIRONMENT: OnceLock<ort::environment::Environment> = OnceLock::new();

fn environment() -> Result<&'static ort::environment::Environment, String> {
    if let Some(env) = ENVIRONMENT.get() {
        return Ok(env);
    }
    let built = ort::environment::init().build().map_err(|e| e.to_string())?;
    // A race here means another caller built one first, and `get()` returns theirs.
    let _ = ENVIRONMENT.set(built);
    ENVIRONMENT.get().ok_or_else(|| String::from("the ONNX Runtime environment could not be initialised"))
}

/// A loaded ONNX graph.
pub struct Model {
    session: OrtSession,
    device: Device,
    loaded: Loaded,
}

impl core::fmt::Debug for Model {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Model").field("loaded", &self.loaded).finish_non_exhaustive()
    }
}

/// **Which device a graph runs on**, and what the build can actually offer — `#19`.
///
/// `#19`'s requirement is that the fallback be **honest**: a CPU embedding is not necessarily the same vector as a CUDA
/// one, so an index built on one and queried on the other can rank differently. That has to be measured and stated
/// rather than assumed, which is why this type reports the path taken rather than accepting a preference silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Device {
    /// The CPU path. Always available, and **fp32**: `#19`'s own note says fp16 on CPU is slow or unsupported depending
    /// on the runtime, so the CPU path uses a different precision than a GPU one, which is precisely why the
    /// cross-device measurement exists.
    Cpu,
    /// A CUDA device, by index. Only reachable in a build with `ort`'s CUDA feature, which this crate does **not** enable
    /// by default — see [`Device::available`].
    Cuda {
        /// The device ordinal, as the runtime numbers it.
        id: i32,
    },
    /// A caller asked for CUDA and this build cannot provide it, so the CPU path ran instead.
    ///
    /// **Reported rather than silent.** A fallback that is invisible is how a developer ends up measuring CPU
    /// throughput while believing it is a GPU figure.
    FellBackFromCuda,
}

impl Device {
    /// What this build can offer, so a caller can decide before loading anything.
    ///
    /// **`Device::Cpu`, unconditionally, and the `cuda` feature is an empty marker rather than a promise.**
    ///
    /// `ort` 2.0-rc.15 — the version this crate pins — has no CUDA feature to enable, and the crate does not build its
    /// own runtime. So there is no CUDA path here to choose, and reporting one would be the silent fallback `#19`
    /// forbids in the other direction: a developer would read *"CUDA"* and measure a CPU figure.
    ///
    /// A project that needs CUDA builds its own binary against an `ort` with its runtime, and constructs the graph
    /// itself; what this crate can promise is that the CPU path says it is the CPU path.
    pub fn available() -> Device {
        Device::Cpu
    }

    /// **The path actually taken**, as a sentence a log or a panel shows.
    pub fn describe(self) -> &'static str {
        match self {
            Device::Cpu => {
                "CPU, fp32 — no CUDA device in this build; an index built on a GPU may rank differently here"
            }
            Device::Cuda { .. } => "CUDA",
            Device::FellBackFromCuda => "CUDA was requested and this build has none, so the CPU path ran",
        }
    }

    /// True when the caller asked for CUDA and did not get it.
    pub fn fell_back(self) -> bool {
        matches!(self, Device::FellBackFromCuda)
    }
}

impl Model {
    /// **Load a graph from a file**, reporting what the runtime says about its axes, on the device this build offers.
    ///
    /// The `dimension` is read from the output shape's last axis, and `dynamic_batch` from whether the first axis is
    /// symbolic — measured facts about the file, not assumptions about the model.
    pub fn load(path: &str) -> Result<Model, LoadProblem> {
        let unreadable = |e: String| LoadProblem::Unreadable { path: path.to_string(), error: e };
        let env = environment().map_err(unreadable)?;
        let session = OrtSession::builder(env)
            .map_err(|e| unreadable(e.to_string()))?
            .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
            .map_err(|e| unreadable(e.to_string()))?
            .commit_from_file(path)
            .map_err(|e| unreadable(e.to_string()))?;

        let inputs: Vec<String> = session.inputs().iter().map(|i| i.name().to_string()).collect();
        let outputs: Vec<String> = session.outputs().iter().map(|o| o.name().to_string()).collect();

        // Read the shape the runtime reports. A symbolic first axis is what `[-1, n]` means in the ONNX file, and it is
        // exactly the difference the audio and text towers showed.
        let mut dimension = 0usize;
        let mut dynamic_batch = false;
        if let Some(out) = session.outputs().first() {
            use ort::value::ValueType;
            if let ValueType::Tensor { shape, .. } = out.dtype() {
                // A symbolic dimension arrives as `-1`; the last axis is the embedding width.
                if let Some(&last) = shape.last() {
                    dimension = if last > 0 { last as usize } else { 0 };
                }
                dynamic_batch = shape.first().is_none_or(|d| *d < 0);
            }
        }

        Ok(Model {
            session,
            device: Device::available(),
            loaded: Loaded { inputs, outputs, dimension, dynamic_batch },
        })
    }

    /// **Which device this model is running on**, so a caller can report the path rather than assume one — `#19`'s
    /// criterion 1.
    pub fn device(&self) -> Device {
        self.device
    }

    /// What the runtime said about the graph.
    pub fn loaded(&self) -> &Loaded {
        &self.loaded
    }

    /// **Embed text.** `ids` and `mask` are the tokeniser's output; this crate does not own a tokeniser.
    ///
    /// Returns the graph's own values, which CLAP already L2-normalises — measured at norm 1.000000 in
    /// `runs/2026-11-03-63/evidence/ort-probe.txt`, so a later cosine is a dot product.
    pub fn embed_text(&mut self, ids: &[i64], mask: &[i64]) -> Result<Vec<f32>, LoadProblem> {
        if ids.len() != mask.len() || ids.is_empty() {
            return Err(LoadProblem::Unreadable {
                path: String::from("<text>"),
                error: format!("{} ids and {} mask values cannot form one row", ids.len(), mask.len()),
            });
        }
        let n = ids.len();
        let shape = vec![1i64, n as i64];
        let mut run = || -> Result<Vec<f32>, String> {
            let ids_t = Tensor::from_array((shape.clone(), ids.to_vec())).map_err(|e| e.to_string())?;
            let mask_t = Tensor::from_array((shape.clone(), mask.to_vec())).map_err(|e| e.to_string())?;
            let outputs = self
                .session
                .run(ort::inputs!["input_ids" => ids_t, "attention_mask" => mask_t])
                .map_err(|e| e.to_string())?;
            let name = self.loaded.outputs.first().cloned().unwrap_or_default();
            let (_, data) = outputs[name.as_str()].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            Ok(data.to_vec())
        };
        run().map_err(|error| LoadProblem::Unreadable { path: String::from("<text>"), error })
    }

    /// **Embed audio.** `features` is [`crate::semantic::mel::fusion_channels`]'s `[4][1001][64]`, and `is_longer` is the
    /// graph's second input.
    ///
    /// The audio tower reports inputs named `input_features` and `is_longer`, and its batch axis is **fixed** — measured
    /// in `runs/2026-11-03-62/research.md`: `patch_embed` bakes the batch into a `Reshape` and any batch but 1 fails. So
    /// this takes exactly one clip, and a caller that wants many calls it once per clip. `Loaded::dynamic_batch` says
    /// which kind of graph was loaded, so this can refuse rather than fail inside the runtime.
    pub fn embed_audio(
        &mut self,
        features: &[Vec<Vec<f32>>],
        is_longer: bool,
    ) -> Result<Vec<f32>, LoadProblem> {
        let unusable = |e: String| LoadProblem::Unreadable { path: String::from("<audio>"), error: e };
        if features.len() != 4 || features.iter().any(|c| c.len() != MODEL_MEL_FRAMES) {
            return Err(unusable(format!(
                "the audio tower takes 4 channels of {MODEL_MEL_FRAMES} frames; got {} channels",
                features.len()
            )));
        }
        let mut flat = Vec::with_capacity(4 * MODEL_MEL_FRAMES * 64);
        for channel in features {
            for frame in channel {
                if frame.len() != 64 {
                    return Err(unusable(format!("a mel frame has {} bands, not 64", frame.len())));
                }
                flat.extend_from_slice(frame);
            }
        }
        let shape = vec![1i64, 4, MODEL_MEL_FRAMES as i64, 64];
        // **`is_longer` is a `bool` tensor, not an `i64` one.** Measured: the graph rejected `int64` with *"Unexpected
        // input data type. Actual: (tensor(int64)) , expected: (tensor(bool))"*. The type is declared by the graph and
        // was guessed from the Python side, where `is_longer` arrives as a bool tensor.
        let longer = vec![is_longer];
        // No `mut`: `embed_audio` clones its captures rather than moving them, so the closure is `Fn`.
        let run = || -> Result<Vec<f32>, String> {
            let feats = Tensor::from_array((shape.clone(), flat)).map_err(|e| e.to_string())?;
            let longer_t = Tensor::from_array((vec![1i64, 1], longer.clone())).map_err(|e| e.to_string())?;
            let outputs = self
                .session
                .run(ort::inputs!["input_features" => feats, "is_longer" => longer_t])
                .map_err(|e| e.to_string())?;
            let name = self.loaded.outputs.first().cloned().unwrap_or_default();
            let (_, data) = outputs[name.as_str()].try_extract_tensor::<f32>().map_err(|e| e.to_string())?;
            Ok(data.to_vec())
        };
        run().map_err(unusable)
    }
}
