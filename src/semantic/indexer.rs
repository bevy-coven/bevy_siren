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
    /// Sidecar records that name a file the scan did not find.
    ///
    /// The commonest cause is a typo or a path spelling that does not match the folder, and it is exactly the case a
    /// silent skip would hide: the licence looks recorded from the sidecar author's point of view and is absent from
    /// every asset's point of view.
    pub unattributed: usize,
    /// A sidecar that exists and could not be read, carrying the reader's own words.
    ///
    /// A run that finds **no** sidecar is normal and leaves this `None`. A sidecar that is present and unreadable is
    /// not: it means every licence in it is missing, and the crate's rule is that such a thing is said rather than
    /// skipped.
    pub sidecar: Option<String>,
    /// How many files embedded but produced no loudness measurement.
    ///
    /// A measurement that fails does not fail the file — the vector is the index's reason to exist — so without this
    /// count a run whose measurements all failed would report complete success and store nothing.
    pub unmeasured: usize,
    /// Whether any file exceeded the model's one window, so the audio tower's `feature fusion` branch ran.
    ///
    /// Reported rather than assumed: the branch changes what the graph computes, and a caller comparing two indexes
    /// needs to know whether one of them took it.
    pub longer: bool,
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
        if self.unmeasured > 0 {
            s.push_str(&format!(", {} unmeasured", self.unmeasured));
        }
        if self.unattributed > 0 {
            s.push_str(&format!(", {} licence records matched no file", self.unattributed));
        }
        if let Some(why) = &self.sidecar {
            s.push_str(&format!(". Licence record unreadable: {why}"));
        }
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

    // **Read the sidecar before anything is embedded**, so a licence is attached to every asset the build produces and
    // an unreadable sidecar is reported even for a folder whose files all embed fine. The first sidecar name that
    // exists wins; a second is a conflict the caller resolves rather than a merge this crate invents.
    let mut unattributed = 0usize;
    let mut sidecar_report: Option<String> = None;
    match read_sidecar(root) {
        Ok(Some(sidecar)) => {
            // A sidecar names **files**; the index is keyed by **content hash**. This is the join, and it is the only
            // place the two identities meet. An asset whose content appears at several paths matches if *any* of them
            // is named, which is why this loops the paths rather than taking the first.
            for asset in library.assets.values() {
                let found = asset.paths.iter().find_map(|relative| {
                    let key = relative.to_string_lossy().replace('\\', "/");
                    sidecar.records.get(&key).or_else(|| sidecar.records.get(&*relative.to_string_lossy()))
                });
                if let Some(provenance) = found {
                    index.provenance.insert(asset.hash, provenance.clone());
                }
            }
            // **Every unused record is counted.** A sidecar with more rows than files is normal for a pack that ships
            // sounds the project did not take — but a row that names a file the folder does not have is the commonest
            // kind of typo, and silently ignoring it makes a licence look recorded that is attached to nothing.
            let named: std::collections::BTreeSet<&str> =
                library.assets.values().flat_map(|a| a.paths.iter()).filter_map(|p| p.to_str()).collect();
            unattributed = sidecar
                .records
                .keys()
                .filter(|key| {
                    !named.contains(key.as_str()) && !named.contains(key.replace('\\', "/").as_str())
                })
                .count();
        }
        Ok(None) => {}
        Err(e) => sidecar_report = Some(e.to_string()),
    }

    // **The scan's skips are split by what they mean to a caller.** A file that is not audio at all is a fact about
    // the folder; a file that *is* audio by extension but would not read is a failure, and must appear in `failed`
    // beside the ones that decoded and would not embed. The first version dropped the second kind on the floor —
    // measured by `a_corrupt_file_is_counted_not_swallowed`, whose bad WAV vanished entirely — which is exactly the
    // silence this crate refuses at every other layer.
    let mut not_audio: Vec<PathBuf> = Vec::new();
    let mut failed: Vec<Failure> = Vec::new();
    // How many files embedded but produced no loudness. Reported rather than swallowed: a run that measures nothing
    // must not look like one that measured everything.
    let mut unmeasured: usize = 0;
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
        match embed_measured(&path, &mut embed) {
            Ok(Analysed { vector, loudness, spectral, tempo, key }) => {
                // The index refuses a vector of the wrong width, and that refusal is a real failure here: the
                // embedder and the stamp disagree about the model.
                if let Err(e) = index.insert(asset.hash, vector) {
                    failed.push(Failure { path: relative.clone(), why: e.to_string() });
                } else if loudness.is_some() || spectral.is_some() || tempo.is_some() || key.is_some() {
                    if let Some(l) = loudness {
                        index.loudness.insert(asset.hash, l);
                    }
                    if let Some(sp) = spectral {
                        index.spectral.insert(asset.hash, sp);
                    }
                    if let Some(t) = tempo {
                        index.tempo.insert(asset.hash, t);
                    }
                    if let Some(k) = key {
                        index.key.insert(asset.hash, k);
                    }
                } else {
                    // A file that embedded but could not be measured is **not** a failure — the index's reason to exist
                    // is the vector. It is counted, so a run that measures nothing says so instead of looking complete.
                    unmeasured += 1;
                }
            }
            Err(why) => failed.push(Failure { path: relative.clone(), why }),
        }
    }

    Ok(Built { index, failed, not_audio, unattributed, sidecar: sidecar_report, unmeasured, longer: false })
}

/// Decode, convert and embed one file, with every step's failure turned into a sentence.
/// **The sidecar this folder ships, if it ships one**, by [`super::provenance::SIDECAR_NAMES`]' order.
///
/// `Ok(None)` means *no sidecar here*, which is normal. `Err` means one exists and could not be read — a different
/// fact, and the caller reports it rather than treating both as "no licences".
fn read_sidecar(
    root: &Path,
) -> Result<Option<super::provenance::Sidecar>, super::provenance::ProvenanceProblem> {
    for name in super::provenance::SIDECAR_NAMES {
        let path = root.join(name);
        if path.is_file() {
            return super::provenance::Sidecar::read(&path).map(Some);
        }
    }
    Ok(None)
}

pub(super) fn embed_one(
    path: &Path,
    embed: &mut impl FnMut(&[f32]) -> Result<Vec<f32>, String>,
) -> Result<Vec<f32>, String> {
    embed_measured(path, embed).map(|a| a.vector)
}

/// **Decode, convert, measure and embed one file**, sharing the decode between the measurement and the embedding.
///
/// The two are deliberately one function. Loudness needs the same mono buffer at the model's rate that the embedder
/// needs, so measuring in a second pass would decode every file twice — the cost the whole feature exists to avoid on a
/// thousand-file library. The buffer is handed to `measure` and to `embed` in turn, and neither keeps it.
///
/// **What one file yielded**: its vector, and whatever the measurement passes could say about it.
///
/// A struct rather than a three-tuple, because the tuple needed a `type` alias to pass clippy and a caller reading
/// `.spectral` is clearer than `.2`. Both measured fields are `Option` because a measurement can fail without the file
/// failing — the vector is the index's reason to exist.
pub(super) struct Analysed {
    /// The embedding.
    pub vector: Vec<f32>,
    /// Its loudness, when the measurement succeeded.
    pub loudness: Option<super::loudness::Loudness>,
    /// Its spectral shape, when the analysis succeeded.
    pub spectral: Option<super::spectral::Spectral>,
    /// Its tempo, when it is rhythmic enough to have one. Present-but-absent-bpm is a real answer: see
    /// [`super::tempo::Tempo::bpm`].
    pub tempo: Option<super::tempo::Tempo>,
    /// Its key **and confidence**, when it has tonal content.
    pub key: Option<super::key::Estimate>,
}

/// **Decode, convert, measure and embed one file**, sharing the decode between the measurement and the embedding.
///
/// The two are deliberately one function. Loudness needs the same mono buffer at the model's rate that the embedder
/// needs, so measuring in a second pass would decode every file twice — the cost the whole feature exists to avoid on a
/// thousand-file library. The buffer is handed to `measure`, to `analyse` and to `embed` in turn, and none keeps it.
///
/// A measurement that fails does **not** fail the file: the embedding is the index's reason to exist and a loudness
/// number or a centroid is an addition to it. The failure is reported in the returned `Option` so a caller can count
/// it, rather than silently dropping the asset's loudness with no record that anything went wrong.
pub(super) fn embed_measured(
    path: &Path,
    embed: &mut impl FnMut(&[f32]) -> Result<Vec<f32>, String>,
) -> Result<Analysed, String> {
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
    // **Measured first, from the same buffer.** `measure` takes the mono signal at the model's rate, which is exactly
    // what the embedder is about to take, so this costs one IIR pass and no second decode.
    let loudness = super::loudness::measure(&mono, super::MODEL_SAMPLE_RATE).ok();
    // **The spectral features come from the same buffer**, for the same reason loudness does: a second decode would
    // double the cost of every file to add three statistics.
    // **One spectral pass, two consumers.** `spectral` and `tempo` walk the same 1024-point frames with the same hop,
    // so computing the transform twice per frame was measured waste — 3.96 + 4.23 ms against a shared ~4 ms. The
    // spectra are computed here and handed to both; key keeps its own 8192-point transform because a chroma needs a
    // finer resolution than a centroid does.
    let spectra = super::analysis::spectra(&mono);
    let spectral = super::spectral::analyse_spectra(&mono, &spectra).ok();
    // **Tempo and key from the same buffer**, for the third and fourth time: a decode per feature would make a
    // thousand-file library four thousand decodes. Both return `Option` because a clip can be too short for tempo's
    // two frames or for key's chroma window, and that is a fact to store rather than an error to raise.
    let tempo = super::tempo::estimate_spectra(&mono, &spectra).ok();
    let key = super::key::estimate(&mono).ok();
    let vector = embed(&mono)?;
    Ok(Analysed { vector, loudness, spectral, tempo, key })
}

/// The file a built index lives in. One name for both formats, because the bytes say which one it is.
pub const INDEX_FILE: &str = "siren-index";

/// **Write a built index beside its library**, so the next run can load it instead of rebuilding.
///
/// The path is `<root>/siren-index.txt`. A caller that wants it elsewhere writes `index.to_text()` itself; this exists
/// so the ordinary case is one call.
pub fn write(root: &Path, index: &Index) -> Result<PathBuf, String> {
    let path = root.join(INDEX_FILE);
    // **The binary form, because ADR 0004 measured it at a third of the size.** A 5 000-clip library is 10.28 MB here
    // against 31.88 MB as text, asserted by `five_thousand_vectors_weigh_what_adr_0004_says` rather than quoted.
    std::fs::write(&path, index.to_bytes()).map_err(|e| format!("cannot write `{}`: {e}", path.display()))?;
    Ok(path)
}

/// **Read the index a previous build wrote**, or say why it cannot be used.
pub fn read(root: &Path, loaded: &ModelStamp) -> Result<Index, String> {
    let path = root.join(INDEX_FILE);
    let bytes = std::fs::read(&path).map_err(|e| format!("cannot read `{}`: {e}", path.display()))?;
    // **The bytes say which format they are**, so an index written as text by an older build still loads. That is what
    // ADR 0004 meant by "distinguishable by its magic rather than its extension": the file name does not decide.
    if bytes.starts_with(super::index::MAGIC_BINARY) {
        Index::from_bytes(&bytes, loaded).map_err(|e| e.to_string())
    } else {
        let text = String::from_utf8(bytes)
            .map_err(|e| format!("`{}` is neither a binary index nor UTF-8 text: {e}", path.display()))?;
        Index::load(&text, loaded).map_err(|e| e.to_string())
    }
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

/// **Build an index with the model**, which is the whole feature in one call: a folder in, a stamped index out.
///
/// [`build`] takes an embedder as a seam so the orchestration is testable without weights. This is the seam filled in:
/// [`super::mel::input_features`] turns each buffer into the audio tower's four channels, and [`super::model::Model::embed_audio`]
/// turns those into a vector. Nothing else is decided here.
///
/// `longer` says whether any clip exceeded the model's window. The model's own extractor sets it per clip; here it is
/// `true` when a file was longer than ten seconds, which is the same condition read from the buffer's length rather
/// than from a flag — and it is reported rather than assumed, because it changes which branch of the audio tower runs.
pub fn build_from_model(
    root: &Path,
    model: &mut super::model::Model,
    filterbank: &super::mel::Filterbank,
    stamp: ModelStamp,
) -> Result<Built, BuildProblem> {
    // A `Cell` rather than a `RefCell`: the closure is `FnMut` and the value is `Copy`, so the flag is written by the
    // embedder and read once afterwards.
    let longer = std::cell::Cell::new(false);
    let mut built = build(root, stamp, |buffer: &[f32]| {
        if buffer.len() > super::mel::MODEL_WINDOW_SAMPLES {
            longer.set(true);
        }
        let features = super::mel::input_features(buffer, filterbank).map_err(|e| e.to_string())?;
        model.embed_audio(&features, longer.get()).map_err(|e| e.to_string())
    })?;
    built.longer = longer.get();
    Ok(built)
}

// **`std::error::Error` as well as `Display`.** Without it a caller writing `?` in a function returning
// `Box<dyn Error>` cannot use any of this crate's failures — measured: a probe on the GPU host failed to compile with
// *"`BuildProblem`: `std::error::Error` is not satisfied"* on three lines in a row. An error a caller has to unwrap by hand is
// half an error.
impl std::error::Error for BuildProblem {}
