//! **Building an index from a folder** — the step that was missing.
//!
//! The pieces all existed and nothing composed them. These tests prove the composition: the order, the content-hash
//! identity, the length the resampler promises, and the failure policy that must count a bad file rather than swallow
//! it.

#![cfg(feature = "semantic")]

use std::path::{Path, PathBuf};

use bevy_siren::semantic::index::ModelStamp;
use bevy_siren::semantic::indexer::{BuildProblem, build, read, stale, vanished, write};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("bevy_siren_indexer_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch");
        Scratch(p)
    }

    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let p = self.0.join(name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("parent");
        }
        std::fs::write(&p, bytes).expect("write");
        p
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

/// A 16-bit mono WAV at `rate` holding `frames` samples of a `freq` Hz tone.
fn wav(rate: u32, freq: f64, frames: usize) -> Vec<u8> {
    let data_len = (frames * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..frames {
        let s = (2.0 * std::f64::consts::PI * freq * i as f64 / f64::from(rate)).sin();
        v.extend_from_slice(&((s * 0.5 * f64::from(i16::MAX)) as i16).to_le_bytes());
    }
    v
}

fn stamp() -> ModelStamp {
    ModelStamp::measured_clap()
}

/// An embedder that records the length of every buffer it is given, so the test can assert the model's rate.
fn recorder<'a>(seen: &'a mut Vec<usize>) -> impl FnMut(&[f32]) -> Result<Vec<f32>, String> + 'a {
    move |buffer: &[f32]| {
        seen.push(buffer.len());
        // A vector of the stamp's width whose first value is the buffer's length, so two files differ.
        let mut v = vec![0.0f32; 512];
        v[0] = buffer.len() as f32;
        Ok(v)
    }
}

#[test]
fn a_folder_of_three_files_becomes_three_indexed_records() {
    let s = Scratch::new("three");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    s.file("b.wav", &wav(48_000, 660.0, 9600));
    s.file("sub/c.wav", &wav(48_000, 220.0, 2400));

    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    assert_eq!(built.indexed(), 3, "three files, three records");
    assert!(built.failed.is_empty(), "nothing failed: {:?}", built.failed);
    assert_eq!(seen.len(), 3, "each file embedded once");
}

#[test]
fn every_buffer_reaches_the_embedder_at_the_models_rate() {
    // The composition claim: decode → downmix → resample happens, and the embedder sees 48 kHz mono. A 44.1 kHz file
    // of 0.5 s must arrive as 24000 samples, not 22050.
    let s = Scratch::new("rate");
    s.file("cd.wav", &wav(44_100, 1000.0, 22_050));

    let mut seen = Vec::new();
    build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    assert_eq!(seen, vec![24_000], "0.5 s at 44.1 kHz is 24000 samples at 48 kHz");
}

#[test]
fn a_corrupt_file_is_counted_not_swallowed() {
    // **The failure policy.** One bad file must not stop a good library, and must not vanish either.
    let s = Scratch::new("corrupt");
    s.file("good.wav", &wav(48_000, 440.0, 4800));
    s.file("broken.wav", b"RIFF\x00\x00\x00\x00WAVE not a real wav");

    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds despite one bad file");
    assert_eq!(built.indexed(), 1, "the good file is indexed");
    assert_eq!(built.failed.len(), 1, "the bad one is counted");
    assert!(built.failed[0].path.ends_with("broken.wav"));
    assert!(!built.failed[0].why.is_empty(), "and the reason is a sentence: {:?}", built.failed[0].why);
    assert!(built.describe().contains("1 indexed"), "{}", built.describe());
    assert!(built.describe().contains("1 failed"), "{}", built.describe());
}

#[test]
fn a_folder_with_no_audio_refuses_rather_than_returning_an_empty_index() {
    // An empty index is indistinguishable from a successful build of an empty folder, so a caller who pointed at the
    // wrong directory would never learn.
    let s = Scratch::new("empty");
    s.file("notes.md", b"# not audio");

    let mut seen = Vec::new();
    let err = build(s.path(), stamp(), recorder(&mut seen)).expect_err("refused");
    match &err {
        BuildProblem::NothingFound { path } => assert!(path.contains("bevy_siren_indexer_empty")),
        other => panic!("expected NothingFound, got {other:?}"),
    }
    assert!(err.to_string().contains("no audio files were found"), "{err}");
}

#[test]
fn a_path_that_is_not_a_directory_refuses_with_the_path() {
    let s = Scratch::new("notdir");
    let file = s.file("a.wav", &wav(48_000, 440.0, 100));
    let mut seen = Vec::new();
    let err = build(&file, stamp(), recorder(&mut seen)).expect_err("refused");
    assert!(matches!(err, BuildProblem::NotADirectory { .. }), "got {err:?}");
    assert!(err.to_string().contains("a.wav"), "the path is in the message: {err}");
}

#[test]
fn the_index_is_written_and_read_back_with_its_stamp() {
    // The whole reason an indexer exists: the work survives the process.
    let s = Scratch::new("persist");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    s.file("b.wav", &wav(48_000, 660.0, 9600));

    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    let written = write(s.path(), &built.index).expect("writes");
    assert!(written.exists(), "the index file is on disk");

    let back = read(s.path(), &stamp()).expect("reads back");
    assert_eq!(back.records.len(), 2);
    assert_eq!(back.stamp, built.index.stamp, "the stamp survives");
}

#[test]
fn a_rebuild_is_what_an_incremental_index_asks_about() {
    // `stale` and `vanished` are the two halves of an incremental update, and they are the reason the content hash is
    // the identity: a renamed file is neither.
    let s = Scratch::new("incremental");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    write(s.path(), &built.index).expect("writes");

    assert!(stale(s.path(), &built.index).is_empty(), "nothing new on disk");

    // Add one file: exactly one stale hash.
    s.file("b.wav", &wav(48_000, 660.0, 2400));
    assert_eq!(stale(s.path(), &built.index).len(), 1, "one new file is one stale hash");
    assert!(vanished(s.path(), &built.index).is_empty(), "nothing vanished");

    // **Rename the asset the index already holds** — `a.wav` — so the claim is about identity and not about a file
    // that was never indexed. The hash is unchanged, so neither list moves.
    std::fs::rename(s.path().join("a.wav"), s.path().join("a_renamed.wav")).expect("rename");
    assert_eq!(stale(s.path(), &built.index).len(), 1, "only `b.wav`, which was never built, is new");
    assert!(vanished(s.path(), &built.index).is_empty(), "and a rename loses nothing");

    // Delete it: now the index holds a hash the disk does not.
    std::fs::remove_file(s.path().join("a_renamed.wav")).expect("remove");
    assert_eq!(vanished(s.path(), &built.index).len(), 1, "one indexed asset is gone");
}

#[test]
fn an_embedder_that_fails_on_one_file_names_that_file() {
    // The embedder is a seam, and a seam can fail. That failure must arrive with the path, not as a missing record.
    let s = Scratch::new("embedfail");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    s.file("b.wav", &wav(48_000, 660.0, 4800));

    let mut calls = 0;
    let built = build(s.path(), stamp(), |_buffer: &[f32]| {
        calls += 1;
        if calls == 2 { Err("the model rejected this buffer".to_string()) } else { Ok(vec![1.0f32; 512]) }
    })
    .expect("builds despite one embed failure");

    assert_eq!(built.indexed(), 1, "one of the two indexed");
    assert_eq!(built.failed.len(), 1);
    assert!(built.failed[0].why.contains("the model rejected this buffer"), "{:?}", built.failed[0].why);
}

#[test]
fn a_vector_of_the_wrong_width_is_a_failure_not_a_panic() {
    // The embedder and the stamp can disagree about the model. `Index::insert` refuses; the indexer turns that refusal
    // into a counted failure with the path.
    let s = Scratch::new("width");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    let built = build(s.path(), stamp(), |_b: &[f32]| Ok(vec![0.0f32; 10])).expect("builds");
    assert_eq!(built.indexed(), 0);
    assert_eq!(built.failed.len(), 1);
    assert!(built.failed[0].why.contains("10"), "the message names the width: {}", built.failed[0].why);
}

#[test]
fn the_same_folder_builds_the_same_index_twice() {
    // Determinism: the composition adds no nondeterminism of its own.
    let s = Scratch::new("determinism");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    s.file("b.wav", &wav(44_100, 660.0, 4410));

    let mut one = Vec::new();
    let mut two = Vec::new();
    let a = build(s.path(), stamp(), recorder(&mut one)).expect("builds");
    let b = build(s.path(), stamp(), recorder(&mut two)).expect("builds");
    assert_eq!(a.index, b.index, "field for field");
    assert_eq!(a.failed, b.failed);
}

#[test]
fn a_file_that_is_not_audio_is_reported_as_such_not_as_a_failure() {
    // The distinction a person needs: "your library has notes.md" is not the same as "a.wav would not decode".
    let s = Scratch::new("notaudio");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    s.file("notes.md", b"# notes");

    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    assert_eq!(built.indexed(), 1);
    assert!(built.failed.is_empty(), "a markdown file is not a failure");
    assert_eq!(built.not_audio.len(), 1);
    assert!(built.describe().contains("1 not audio"), "{}", built.describe());
}
