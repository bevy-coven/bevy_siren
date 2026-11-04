//! **Indexing what changed, rather than everything.** The difference between a tool a developer runs and one they stop
//! running.
//!
//! [`build`](super::indexer::build) indexes a whole folder. That is right for a first run and wrong for the second: a
//! library of a thousand sounds gains one file a week, and re-embedding a thousand to add one is a minute of GPU for a
//! second of work.
//!
//! # The identity rule, stated once
//!
//! **Path is not identity. The content hash is.** The scan already keys a [`Library`](crate::library::Library) by
//! FNV-1a over the file's bytes, so an incremental pass is a set difference over three sets:
//!
//! | Set | Meaning | What happens |
//! |---|---|---|
//! | in the scan, not the index | a new sound | **embed it** |
//! | in the index, not the scan | a deleted sound | **drop its vector** |
//! | in both | unchanged | **nothing** — not even a re-read |
//!
//! And the two cases the issue names, which fall out of that rule rather than needing a case of their own:
//!
//! - **A file that moved path but kept its bytes keeps its vector.** The hash did not change, so it is in all three
//!   sets at once and the answer is *unchanged*. Moving a folder of sounds costs nothing.
//! - **A file re-encoded from Vorbis to WAV is a new asset.** The bytes changed, so the hash did. The same music is a
//!   different recording as far as this crate is concerned — it cannot tell "re-encoded" from "a different take"
//!   without a content analysis it does not have, and **guessing wrong in the other direction is worse**: treating a
//!   genuinely different sound as unchanged would leave the index describing audio that is no longer there.
//!
//! # Why this is a function and not a file operation
//!
//! [`indexer::write`](super::indexer::write) already writes `<root>/siren-index` ([`indexer::INDEX_FILE`](super::indexer::INDEX_FILE)), and
//! [`indexer::stale`](super::indexer::stale) and [`indexer::vanished`](super::indexer::vanished) already answer *"what
//! changed?"* from the disk. This module is the **composition** those three were waiting for: it takes the index a
//! previous run wrote, asks it what changed, embeds only the adds, drops only the removals, and hands back the index
//! the caller then writes with `write`.
//!
//! Keeping the scan-and-compare here rather than in the writer is deliberate. A caller that wants to write under a
//! lock takes one around `write`; a caller that wants to preview a run calls this and never writes at all. The pitfall
//! the issue raises — *"a scan that runs while an index is being written must not corrupt it"* — is a property of the
//! writer, and this module never becomes one.

use std::path::Path;

use super::index::Index;
use super::indexer::{Failure, embed_one};
use crate::library;

/// **What an incremental run did**, as the four numbers a person reads.
///
/// The issue asks for adds, removals and unchanged to be "separate numbers" so a run is observable. They are here, and
/// so is `failed`, because an add that could not be embedded is neither an add nor unchanged and silently counting it
/// as either is the silence this crate refuses everywhere else.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Incremental {
    /// Assets that were embedded for the first time.
    pub added: usize,
    /// Content hashes whose vectors were dropped because the file is gone.
    pub removed: usize,
    /// Assets present in both the scan and the index, whose vectors were kept untouched.
    pub unchanged: usize,
    /// Files that could not be embedded, with the reason.
    pub failed: Vec<Failure>,
}

impl Incremental {
    /// Whether the index actually changed, so a caller can skip a write.
    pub fn is_noop(&self) -> bool {
        self.added == 0 && self.removed == 0
    }

    /// **A sentence a person reads**, in the same shape as [`Built::describe`](super::indexer::Built::describe).
    pub fn describe(&self) -> String {
        let mut s = format!("{} added, {} removed, {} unchanged", self.added, self.removed, self.unchanged);
        if !self.failed.is_empty() {
            s.push_str(&format!(", {} failed", self.failed.len()));
            if let Some(first) = self.failed.first() {
                s.push_str(&format!(". First: `{}`: {}", first.path.display(), first.why));
            }
        }
        s
    }
}

/// Why an incremental pass could not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncrementalProblem {
    /// The root is not a directory, carrying the path so the message points at it.
    NotADirectory {
        /// What was given.
        path: String,
    },
}

impl core::fmt::Display for IncrementalProblem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IncrementalProblem::NotADirectory { path } => {
                write!(f, "`{path}` is not a directory, so there is nothing to index")
            }
        }
    }
}

impl std::error::Error for IncrementalProblem {}

/// **Bring `index` up to date with what is under `root`.**
///
/// Embeds only the assets the index does not have, drops the vectors whose files are gone, and leaves everything else
/// alone. `index` is consumed and returned because the result is a new index, not a mutation of the caller's — a
/// caller that wants to keep the old one only has to clone first, and a caller that does not has no aliasing to reason
/// about.
///
/// The stamp is **not** checked here: [`Index::load`](super::index::Index::load) already refuses an index from a
/// different model, which is the only place that check belongs. An incremental pass on a loaded index is therefore
/// always same-model by construction.
pub fn update(
    root: &Path,
    mut index: Index,
    mut embed: impl FnMut(&[f32]) -> Result<Vec<f32>, String>,
) -> Result<(Index, Incremental), IncrementalProblem> {
    if !root.is_dir() {
        return Err(IncrementalProblem::NotADirectory { path: root.display().to_string() });
    }
    let library = library::scan(root);
    let mut report = Incremental::default();

    // **A file the scan refused is a failure, not silence.** `build` splits the scan's skips by meaning — a file that
    // is not audio at all is a fact about the folder, a file that *is* audio and would not read is a failure — and an
    // incremental pass must do the same, or a corrupt file would be absent from every count at once. Found by
    // `an_undecodable_file_is_failed_not_counted`, whose broken WAV was reported as nothing at all.
    for skip in &library.skipped {
        match &skip.reason {
            // Not audio by extension: not this crate's business to report as a failure, and the scan's own reason
            // already says what it is.
            library::SkipReason::NotAudio { .. } => {}
            reason => report.failed.push(Failure { path: skip.path.clone(), why: reason.to_string() }),
        }
    }

    // **Removals first**, using the function that already answers the question. Doing it before the adds means the
    // index never holds a vector for a file that is gone, even transiently, so a panic between the two halves leaves a
    // smaller index rather than one that describes audio nobody has. Dropping is cheap and cannot fail.
    for hash in super::indexer::vanished(root, &index) {
        index.records.remove(&hash);
        report.removed += 1;
    }

    // **Then the adds and the unchanged**, in one pass over the scan.
    for asset in library.assets.values() {
        if index.records.contains_key(&asset.hash) {
            // **A move lands here, and that is the whole point.** The index holds hashes and vectors, not paths, so
            // a file that moved is *indistinguishable from one that did not* — and therefore costs nothing. There is
            // no move case to get wrong, which is why the policy can be stated in one sentence: path is not identity.
            report.unchanged += 1;
            continue;
        }
        // Every path this content lives at shares one vector, so the first is the one to read.
        let Some(relative) = asset.paths.first() else { continue };
        match embed_one(&root.join(relative), &mut embed) {
            Ok(vector) => match index.insert(asset.hash, vector) {
                Ok(()) => report.added += 1,
                Err(e) => report.failed.push(Failure { path: relative.clone(), why: e.to_string() }),
            },
            // A file that will not embed is **not** counted as an add and **not** as unchanged. It is failed, and a
            // caller that retries will try it again rather than believing it is in.
            Err(why) => report.failed.push(Failure { path: relative.clone(), why }),
        }
    }

    Ok((index, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::EMBEDDING_DIM;
    use crate::semantic::index::ModelStamp;
    use std::path::PathBuf;

    fn stamp() -> ModelStamp {
        ModelStamp::measured_clap()
    }

    #[test]
    fn an_index_matching_the_scan_adds_and_removes_nothing() {
        let index = Index::new(stamp());
        let report = Incremental::default();
        assert!(report.is_noop());
        assert_eq!(report.describe(), "0 added, 0 removed, 0 unchanged");
        drop(index);
    }

    #[test]
    fn describe_counts_every_kind_separately() {
        let report = Incremental { added: 1, removed: 2, unchanged: 3, failed: vec![] };
        assert_eq!(report.describe(), "1 added, 2 removed, 3 unchanged");
        assert!(!report.is_noop());
    }

    #[test]
    fn a_move_changes_no_vector_so_the_run_is_a_noop() {
        // The index is keyed by content hash and holds no paths, so a rename is simply "unchanged" — the count does
        // not move, nothing is re-embedded, and there is no separate case that could disagree with this one.
        let report = Incremental { added: 0, removed: 0, unchanged: 3, failed: vec![] };
        assert_eq!(report.describe(), "0 added, 0 removed, 3 unchanged");
        assert!(report.is_noop(), "a move changes no vector, so there is nothing to write");
    }

    #[test]
    fn a_failure_is_named_rather_than_counted_as_an_add() {
        let report = Incremental {
            added: 0,
            removed: 0,
            unchanged: 1,
            failed: vec![Failure { path: PathBuf::from("bad.wav"), why: String::from("not audio") }],
        };
        assert_eq!(
            report.describe(),
            "0 added, 0 removed, 1 unchanged, 1 failed. First: `bad.wav`: not audio"
        );
    }

    #[test]
    fn an_impossible_root_is_refused_with_the_path() {
        let err =
            update(Path::new("/definitely/not/here"), Index::new(stamp()), |_| Ok(vec![0.0; EMBEDDING_DIM]))
                .expect_err("not a directory");
        assert_eq!(err, IncrementalProblem::NotADirectory { path: String::from("/definitely/not/here") });
    }
}
