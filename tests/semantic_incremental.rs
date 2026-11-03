//! **Indexing only what changed** — issue #16's four acceptance criteria, against real files on disk.
//!
//! The unit tests in `incremental.rs` prove the *report*; these prove the *behaviour*, because every criterion is a
//! claim about what happens to a folder between two runs:
//!
//! | # | Criterion | Test |
//! |---|---|---|
//! | 1 | adding one file to a 100-file library embeds **exactly one** | [`adding_one_file_embeds_exactly_one`] |
//! | 2 | deleting a file removes its vector, so a query no longer returns it | [`a_deleted_file_leaves_no_vector`] |
//! | 3 | renaming a file keeps its vector, count unchanged | [`a_rename_costs_nothing`] |
//! | 4 | a run reports adds, removals and unchanged separately | [`every_run_reports_the_three_counts`] |
//!
//! The last one also covers the case the issue calls *"a real decision"*: a file re-encoded to different bytes is a
//! **new** asset, and the test states that rather than leaving it to be inferred.

#![cfg(feature = "semantic")]

use std::path::{Path, PathBuf};

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::incremental::update;
use bevy_siren::semantic::index::{Index, ModelStamp};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("bevy_siren_incremental_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch");
        Scratch(p)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    /// A valid mono 48 kHz WAV whose sample values are derived from `seed`, so two different seeds are two different
    /// files with two different content hashes.
    fn wav(&self, name: &str, seed: u32) -> PathBuf {
        let path = self.0.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(&path, wav_bytes(seed)).expect("write");
        path
    }
}

/// A minimal mono 16-bit 48 kHz WAV. The bytes matter more than the audio: the content hash is over the file, so a
/// different `seed` is a different asset however alike the two sound.
fn wav_bytes(seed: u32) -> Vec<u8> {
    let rate = 48_000u32;
    let samples: Vec<i16> = (0..2_400)
        .map(|i| {
            let t = (i as f32 + seed as f32 * 7.0) / rate as f32;
            ((t * 440.0 * std::f32::consts::TAU).sin() * 8_000.0) as i16
        })
        .collect();
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    out.extend_from_slice(&2u16.to_le_bytes()); // block align
    out.extend_from_slice(&16u16.to_le_bytes()); // bits
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// The embedding the tests use: a deterministic unit vector that depends on the samples, so two different files embed
/// differently and the same file embeds the same twice.
///
/// It counts its calls through the returned `Cell`, which is how "embeds exactly one file" becomes an assertion about a
/// **count** rather than an inference from the index length.
fn counting_embedder(calls: &std::cell::Cell<usize>) -> impl FnMut(&[f32]) -> Result<Vec<f32>, String> + '_ {
    move |buffer: &[f32]| {
        calls.set(calls.get() + 1);
        let mut seed = buffer.len() as f32;
        for (i, s) in buffer.iter().enumerate().take(64) {
            seed += s * (i as f32 + 1.0);
        }
        let mut v: Vec<f32> = (0..EMBEDDING_DIM).map(|i| (seed + i as f32).sin()).collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in v.iter_mut() {
            *x /= norm;
        }
        Ok(v)
    }
}

fn stamp() -> ModelStamp {
    ModelStamp::measured_clap()
}

/// **Criterion 1: adding one file to a 100-file library embeds exactly one.**
///
/// The count is taken from the embedder's own call count, so an implementation that re-embeds everything and then
/// discards the extras cannot pass by producing the right index.
#[test]
fn adding_one_file_embeds_exactly_one() {
    let scratch = Scratch::new("add_one");
    for i in 0..100 {
        scratch.wav(&format!("sound-{i:03}.wav"), i);
    }

    let calls = std::cell::Cell::new(0);
    let (index, first) =
        update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("first");
    assert_eq!(first.added, 100, "a first run indexes everything");
    assert_eq!(first.removed, 0);
    assert_eq!(first.unchanged, 0);
    assert_eq!(calls.get(), 100, "one embed per file");

    // Add exactly one.
    scratch.wav("sound-new.wav", 999);
    calls.set(0);
    let (index, second) = update(scratch.path(), index, counting_embedder(&calls)).expect("second");

    assert_eq!(calls.get(), 1, "adding one file to a hundred embeds EXACTLY one");
    assert_eq!(second.added, 1);
    assert_eq!(second.removed, 0);
    assert_eq!(second.unchanged, 100);
    assert_eq!(index.len(), 101);
}

/// **Criterion 2: deleting a file removes its vector, asserted by a query that no longer returns it.**
///
/// The assertion is on the index's contents rather than on a count, because a count would pass for an implementation
/// that dropped the *wrong* vector.
#[test]
fn a_deleted_file_leaves_no_vector() {
    let scratch = Scratch::new("delete");
    for i in 0..5 {
        scratch.wav(&format!("keep-{i}.wav"), i);
    }
    let doomed = scratch.wav("doomed.wav", 42);

    let calls = std::cell::Cell::new(0);
    let (index, _) = update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("build");
    assert_eq!(index.len(), 6);
    let doomed_hash = bevy_siren::library::fnv1a(&std::fs::read(&doomed).expect("read"));
    assert!(index.records.contains_key(&doomed_hash), "the doomed file was indexed");

    std::fs::remove_file(&doomed).expect("delete");
    calls.set(0);
    let (index, report) = update(scratch.path(), index, counting_embedder(&calls)).expect("update");

    assert_eq!(report.removed, 1, "one removal");
    assert_eq!(report.added, 0);
    assert_eq!(report.unchanged, 5);
    assert_eq!(calls.get(), 0, "a removal embeds nothing");
    assert!(!index.records.contains_key(&doomed_hash), "and its vector is gone, not merely unreferenced");
    assert_eq!(index.len(), 5);
}

/// **Criterion 3: renaming a file keeps its vector, with the embedding count unchanged.**
///
/// This is the rule stated in the module doc — *path is not identity* — measured rather than asserted in prose.
#[test]
fn a_rename_costs_nothing() {
    let scratch = Scratch::new("rename");
    for i in 0..3 {
        scratch.wav(&format!("track-{i}.wav"), i);
    }
    let calls = std::cell::Cell::new(0);
    let (index, _) = update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("build");
    let before = index.len();
    let vectors: Vec<Vec<f32>> = index.records.values().cloned().collect();

    std::fs::rename(scratch.path().join("track-1.wav"), scratch.path().join("renamed.wav")).expect("rename");

    calls.set(0);
    let (index, report) = update(scratch.path(), index, counting_embedder(&calls)).expect("update");

    assert_eq!(calls.get(), 0, "a rename embeds nothing");
    assert_eq!(report.added, 0);
    assert_eq!(report.removed, 0);
    assert_eq!(report.unchanged, before, "the count is unchanged, which is the criterion");
    assert_eq!(index.len(), before);
    let mut after: Vec<Vec<f32>> = index.records.values().cloned().collect();
    let mut expected = vectors;
    after.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    expected.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
    assert_eq!(
        after, expected,
        "every vector is the same vector, not a re-embedding that happens to be close"
    );
}

/// **Criterion 4: adds, removals and unchanged are separate numbers** — and the re-encode decision, stated.
///
/// A file whose bytes change is a **new asset**, even when it is the same music. That is the issue's own *"real
/// decision"*, and it is asserted here so a later change to merge re-encodings has to delete a test rather than
/// quietly alter behaviour.
#[test]
fn every_run_reports_the_three_counts() {
    let scratch = Scratch::new("counts");
    scratch.wav("a.wav", 1);
    scratch.wav("b.wav", 2);
    let calls = std::cell::Cell::new(0);
    let (index, first) =
        update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("first");
    assert_eq!((first.added, first.removed, first.unchanged), (2, 0, 0));
    assert!(!first.is_noop());

    // One added, one removed, one kept, in a single run.
    scratch.wav("c.wav", 3);
    std::fs::remove_file(scratch.path().join("a.wav")).expect("delete");
    calls.set(0);
    let (index, mixed) = update(scratch.path(), index, counting_embedder(&calls)).expect("mixed");
    assert_eq!((mixed.added, mixed.removed, mixed.unchanged), (1, 1, 1));
    assert_eq!(mixed.describe(), "1 added, 1 removed, 1 unchanged");

    // **The re-encode decision.** Same music, different bytes: `b.wav` is rewritten as a louder WAV, which is a new
    // content hash. It counts as an add and a removal, never as "unchanged".
    scratch.wav("b.wav", 77);
    calls.set(0);
    let (_index, reencoded) =
        update(scratch.path(), mixed_index(index), counting_embedder(&calls)).expect("reencode");
    assert_eq!(reencoded.added, 1, "a re-encoded file is a NEW asset");
    assert_eq!(reencoded.removed, 1, "and the old bytes' vector is gone");
    assert_eq!(reencoded.unchanged, 1, "only the untouched file is unchanged");
    assert_eq!(calls.get(), 1, "one embed, for the new bytes");
}

/// The index a previous assertion produced, handed on unchanged. A named helper so the re-encode block reads as one
/// step rather than as a second update in the middle of the first.
fn mixed_index(index: Index) -> Index {
    index
}

/// **A run that changes nothing is a no-op**, and says so, so a caller can skip the write.
#[test]
fn a_second_run_over_an_unchanged_folder_writes_nothing() {
    let scratch = Scratch::new("noop");
    for i in 0..4 {
        scratch.wav(&format!("s{i}.wav"), i);
    }
    let calls = std::cell::Cell::new(0);
    let (index, _) = update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("build");
    calls.set(0);
    let (_index, again) = update(scratch.path(), index, counting_embedder(&calls)).expect("again");
    assert!(again.is_noop(), "nothing changed, so there is nothing to write");
    assert_eq!(again.describe(), "0 added, 0 removed, 4 unchanged");
    assert_eq!(calls.get(), 0, "and nothing was embedded");
}

/// **A file that will not embed is neither an add nor unchanged**, so a retry tries it again.
#[test]
fn an_undecodable_file_is_failed_not_counted() {
    let scratch = Scratch::new("failed");
    scratch.wav("good.wav", 1);
    std::fs::write(scratch.path().join("broken.wav"), b"not a wav at all").expect("write");

    let calls = std::cell::Cell::new(0);
    let (index, report) =
        update(scratch.path(), Index::new(stamp()), counting_embedder(&calls)).expect("build");
    assert_eq!(report.added, 1, "the good file was indexed");
    assert_eq!(report.failed.len(), 1, "and the broken one is a named failure");
    assert_eq!(report.failed[0].path, PathBuf::from("broken.wav"));
    assert_eq!(index.len(), 1);
}
