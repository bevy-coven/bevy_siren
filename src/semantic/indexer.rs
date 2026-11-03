//! **Building an index from a folder.** The step that was missing.
//!
//! Every piece existed and nothing composed them: [`crate::library::scan`] walked a directory, `decode` turned a file
//! into samples, `resample` put them at the model's rate, \[`super::model::Model`\] embedded, [`Index`] held the vectors and had a
//! stamped, checksummed file format. But `Index::new` plus `Index::insert` is not an indexer — a caller who wanted an
//! index of their library had to write the loop, and get the content hash, the skip list, the failure policy and the
//! stamp right on their own.
//!
//! This module is that loop, in one place.
//!
//! # The embedding is a seam, and it is a *function*
//!
//! Audio embedding is not in this crate yet: it needs a mel spectrogram, which needs a filterbank, which is its own
//! piece of work with its own numbers to get right. Rather than pretend, [`build`] takes an [`EmbedAudio`] — a function
//! from a decoded buffer to a vector — so the orchestration is complete and testable now, and the mel path drops into
//! the one place it belongs later.
//!
//! A function rather than a trait, because there is exactly one thing to implement and a trait would invite a second
//! implementation nobody asked for.
//!
//! # Failures are counted, never swallowed
//!
//! A library is not all-or-nothing: one corrupt file must not stop an index of a thousand. So a file that will not
//! decode is recorded in [`Built::failed`] with its path and the decoder's own words, and the build continues. What it
//! **never** does is skip a file silently — that is the same fault as a refusal without a reason, one layer down.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::library;

use super::buffer::BufferProblem;
use super::decode::{DecodeProblem, decode};
use super::index::{Index, ModelStamp};
use super::resample;

/// **How to embed a mono buffer at the model's rate.** The seam that waits for the mel path.
///
/// Returned as `Result` so an embedding failure is a named failure rather than a zero vector: a zero vector has no
/// cosine with anything, and `top_k` would refuse it later with a message about the *vector* rather than about the file.
pub type EmbedAudio<'a> = dyn FnMut(&[f32]) -> Result<Vec<f32>, String> + 'a;

/// One file that could not be indexed, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// Path relative to the scan root.
    pub path: PathBuf,
    /// What went wrong, in the decoder's or the embedder's own words.
    pub why: String,
}

/// What a build did.
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    /// The index, ready to write or to query.
    pub index: Index,
    /// Files that were found and could not be indexed.
    pub failed: Vec<Failure>,
    /// Files the scan did not treat as audio at all, with the scan's own reason.
    ///
    /// Carried through so a caller can say *"1,204 indexed, 12 not audio, 3 failed"* in one line, which is the
    /// difference between a build a person trusts and one they have to re-run to understand.
    pub not_audio: Vec<PathBuf>,
}

impl Built {
    /// How many files were indexed.
    pub fn indexed(&self) -> usize {
        self.index.len()
    }

    /// **A sentence a person reads**, naming the counts and — when there are any — the first failure.
    ///
    /// The first failure rather than all of them, because a build over a thousand files with a hundred corrupt ones
    /// should not produce a hundred-line wall; `failed` carries the rest for a caller that wants them.
    pub fn describe(&self) -> String {
        let mut s = format!("{} indexed", self.indexed());
        if !self.not_audio.is_empty() {
            s.push_str(&format!(", {} not audio", self.not_audio.len()));
        }
        if !self.failed.is_empty() {
            s.push_str(&format!(", {} failed", self.failed.len()));
            if let Some(first) = self.failed.first() {
                s.push_str(&format!(". First: `{}`: {}", first.path.display(), first.why));
            }
        }
        s
    }
}

/// Why a build could not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildProblem {
    /// The root is not a directory.
    NotADirectory {
        /// The path, so the message points at it.
        path: String,
    },
    /// The scan found no audio at all, so there is nothing to index.
    ///
    /// Refused rather than returning an empty index, because an empty index is indistinguishable from a successful
    /// build of an empty folder, and a caller who pointed at the wrong directory would never learn.
    NothingFound {
        /// The path scanned.
        path: String,
    },
}

impl core::fmt::Display for BuildProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BuildProblem::NotADirectory { path } => {
                write!(f, "`{path}` is not a directory, so there is nothing to index")
            }
            BuildProblem::NothingFound { path } => write!(
                f,
                "no audio files were found under `{path}`. Check the folder, and note that a library of `.mp3` files \
                 is indexed but a folder of `.txt` is not"
            ),
        }
    }
}

/// **Build an index from every audio file under `root`.**
///
/// The steps, in order, for each file the scan found:
///
/// 1. **decode** it, through [`decode`], which reads the rate from the stream rather than from the header;
/// 2. **resample** to the model's rate, through [`resample`], which flushes its tail — a 1.000 s file stays 1.000 s;
/// 3. **embed** it with the caller's [`EmbedAudio`];
/// 4. **insert** it under the content hash the scan gave it, so a rename does not change its identity.
///
/// `root` must be a directory and must contain at least one audio file, or the build refuses. Anything that goes wrong
/// **per file** is recorded in [`Built::failed`] and the build continues.
pub fn build(
    root: &Path,
    stamp: ModelStamp,
    mut embed: impl FnMut(&[f32]) -> Result<Vec<f32>, String>,
) -> Result<Built, BuildProblem> {
    let shown = root.display().to_string();
    if !root.is_dir() {
        return Err(BuildProblem::NotADirectory { path: shown });
    }

    let library = library::scan(root);
    if library.is_empty() {
        return Err(BuildProblem::NothingFound { path: shown });
    }

    let mut index = Index::new(stamp);
    // **The scan's skips are split by what they mean to a caller.** A file that is not audio at all is a fact about
    // the folder; a file that *is* audio by extension but would not read is a failure, and must appear in `failed`
    // beside the ones that decoded and would not embed. The first version dropped the second kind on the floor —
    // measured by `a_corrupt_file_is_counted_not_swallowed`, whose bad WAV vanished entirely — which is exactly the
    // silence this crate refuses at every other layer.
    let mut not_audio: Vec<PathBuf> = Vec::new();
    let mut failed: Vec<Failure> = Vec::new();
    for skip in &library.skipped {
        match &skip.reason {
            library::SkipReason::NotAudio { .. } => not_audio.push(skip.path.clone()),
            reason => failed.push(Failure { path: skip.path.clone(), why: reason.to_string() }),
        }
    }

    for asset in library.assets.values() {
        // A content hash can appear at several paths; the first is the one to read.
        let Some(relative) = asset.paths.first() else { continue };
        let path = root.join(relative);
        match embed_one(&path, &mut embed) {
            Ok(vector) => {
                // The index refuses a vector of the wrong width, and that refusal is a real failure here: the
                // embedder and the stamp disagree about the model.
                if let Err(e) = index.insert(asset.hash, vector) {
                    failed.push(Failure { path: relative.clone(), why: e.to_string() });
                }
            }
            Err(why) => failed.push(Failure { path: relative.clone(), why }),
        }
    }

    Ok(Built { index, failed, not_audio })
}

/// Decode, convert and embed one file, with every step's failure turned into a sentence.
fn embed_one(
    path: &Path,
    embed: &mut impl FnMut(&[f32]) -> Result<Vec<f32>, String>,
) -> Result<Vec<f32>, String> {
    let decoded = decode(path).map_err(|e: DecodeProblem| e.to_string())?;
    let mono = match (decoded.sample_rate, decoded.channels) {
        (0, _) => return Err(BufferProblem::ZeroSampleRate.to_string()),
        (_, 0) => return Err(BufferProblem::NoChannels.to_string()),
        (rate, channels) => {
            // One path from interleaved samples to a model buffer, the same `downmix` + `resample` pair the decode
            // module uses — a second path here would be a second set of rules.
            let mono =
                super::buffer::downmix_to_mono(&decoded.samples, channels).map_err(|e| e.to_string())?;
            if mono.is_empty() {
                return Err(BufferProblem::Empty.to_string());
            }
            resample::to_model_rate(&mono, rate).map_err(|e| e.to_string())?
        }
    };
    embed(&mono)
}

/// **Write a built index beside its library**, so the next run can load it instead of rebuilding.
///
/// The path is `<root>/siren-index.txt`. A caller that wants it elsewhere writes `index.to_text()` itself; this exists
/// so the ordinary case is one call.
pub fn write(root: &Path, index: &Index) -> Result<PathBuf, String> {
    let path = root.join("siren-index.txt");
    std::fs::write(&path, index.to_text()).map_err(|e| format!("cannot write `{}`: {e}", path.display()))?;
    Ok(path)
}

/// **Read the index a previous build wrote**, or say why it cannot be used.
pub fn read(root: &Path, loaded: &ModelStamp) -> Result<Index, String> {
    let path = root.join("siren-index.txt");
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    Index::load(&text, loaded).map_err(|e| e.to_string())
}

/// **Which files a rebuild would touch**, by content hash — the question an incremental index asks.
///
/// Returns the hashes present on disk and absent from the index. Empty means nothing changed.
pub fn stale(root: &Path, index: &Index) -> Vec<u64> {
    let library = library::scan(root);
    library.assets.keys().copied().filter(|h| !index.records.contains_key(h)).collect()
}

/// **Which indexed assets are no longer on disk**, by content hash — the other half of an incremental update.
pub fn vanished(root: &Path, index: &Index) -> Vec<u64> {
    let library = library::scan(root);
    let present: BTreeMap<u64, ()> = library.assets.keys().map(|h| (*h, ())).collect();
    index.records.keys().copied().filter(|h| !present.contains_key(h)).collect()
}
