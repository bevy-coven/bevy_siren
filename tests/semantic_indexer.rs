//! **Building an index from a folder** — the step that was missing.
//!
//! The pieces all existed and nothing composed them. These tests prove the composition: the order, the content-hash
//! identity, the length the resampler promises, and the failure policy that must count a bad file rather than swallow
//! it.

#![cfg(feature = "semantic")]

use std::path::{Path, PathBuf};

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer::{BuildProblem, Built, build, read, stale, vanished, write};

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        // **One directory per call, whatever the label.** Two tests that chose the same label shared one directory, and each one's `remove_dir_all` took the other's files away
        // mid-test: `a_folder_of_every_format…` and `a_written_index_is_binary…` both said `"formats"` and, run together, failed ten times in ten.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p =
            std::env::temp_dir().join(format!("bevy_siren_indexer_{name}_{}_{unique}", std::process::id()));
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

/// **A folder of every format the scan calls audio indexes.** The crate's own error message promises it — *"a library of `.mp3` files
/// is indexed"* — and no test had ever built a folder of anything but WAV, so `.mp3` and `.flac` were scanned as audio and then
/// refused as undecodable by a decoder that had never been given the features to read them.
///
/// Six files of six different byte contents (identical bytes would be one asset): the embedder must be handed every one of them, at
/// the model's rate and the right duration for its length.
#[test]
fn a_folder_of_every_format_the_scan_calls_audio_indexes() {
    let s = Scratch::new("formats");
    s.file("a.wav", &wav(48_000, 440.0, 24_000));
    s.file("b.ogg", include_bytes!("fixtures/tone.ogg"));
    s.file("c.oga", include_bytes!("fixtures/tone-441.ogg"));
    s.file("d.flac", include_bytes!("fixtures/tone.flac"));
    s.file("e.mp3", include_bytes!("fixtures/tone.mp3"));
    s.file("f.flac", include_bytes!("fixtures/tone-441.flac"));

    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");
    assert!(built.failed.is_empty(), "every format the scan calls audio decodes: {:?}", built.failed);
    assert_eq!(built.indexed(), 6, "six files, six records");

    let (short, long): (Vec<usize>, Vec<usize>) = seen.iter().copied().partition(|n| *n < 10_000);
    assert_eq!(short.len(), 2, "the two 0.1 s files (44.1 kHz, converted): {seen:?}");
    // FLAC is exact (0.1 s at 48 kHz is 4800 samples); the Ogg stream declares 4416 frames at 44.1 kHz, which is 4807 at 48 kHz — and
    // *not* the 5573 an uncut Vorbis tail gave (`semantic_decode::an_ogg_is_cut_to_the_length_its_last_page_declares`).
    assert!(short.contains(&4_800), "the FLAC is exact: {short:?}");
    assert!(
        short.iter().all(|n| (4_795..=4_815).contains(n)),
        "0.1 s at 48 kHz is ~4800 samples, no more: {short:?}"
    );
    assert_eq!(long.len(), 4, "the four 0.5 s files: {seen:?}");
    assert!(
        long.iter().all(|n| (22_800..=25_200).contains(n)),
        "0.5 s at 48 kHz is 24000 samples, give or take codec padding: {long:?}"
    );
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

// ---------------------------------------------------------------------------------------------------------------
// The whole feature: a folder in, a stamped index out, with the model doing the embedding.
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn a_folder_builds_an_index_with_the_models_own_spectrogram() {
    // **The chain, end to end**: decode → resample → mel → embed → insert. The model here is the 16 KB fixture, which
    // has the text tower's shape contract; `mel::input_features` is given a stand-in audio tower below, so what this
    // proves is that the *mel path* feeds the *embedder* and the result enters the index — not that the 116 MB audio
    // tower runs, which needs `ort` on `bmb`.
    use bevy_siren::semantic::mel::{self, Filterbank};

    let s = Scratch::new("full");
    s.file("a.wav", &wav(48_000, 440.0, 48_000));
    s.file("b.wav", &wav(44_100, 660.0, 22_050));

    let weights: Vec<f32> =
        include_str!("fixtures/clap-mel-filters.txt").lines().filter_map(|l| l.trim().parse().ok()).collect();
    let filterbank = Filterbank::from_weights(weights).expect("the extractor's own 64 by 513");

    // Stand in for `Model::embed_audio`: run the mel path, then summarise the four channels into a 512-d vector so two
    // different files give two different vectors.
    let built = build(s.path(), stamp(), |buffer: &[f32]| {
        let features = mel::input_features(buffer, &filterbank).map_err(|e| e.to_string())?;
        let mut v = vec![0.0f32; 512];
        for (c, channel) in features.iter().enumerate() {
            for (f, frame) in channel.iter().enumerate() {
                for (b, value) in frame.iter().enumerate() {
                    v[(c * 7 + f * 3 + b) % 512] += value;
                }
            }
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut v {
                *x /= norm;
            }
        }
        Ok(v)
    })
    .expect("builds");

    assert_eq!(built.indexed(), 2, "both files indexed through the whole chain");
    assert!(built.failed.is_empty(), "{:?}", built.failed);
    assert!(!built.longer, "a 1 s and a 0.5 s file are both inside the model's window");
}

/// **A build measures every file it embeds, from the same decode.**
///
/// This is the wiring `#21` needed: `loudness::measure` existed and was tested since iteration 77, but nothing called
/// it during indexing. A three-second 1 kHz tone at 0.5 amplitude is the clip the reference measures at **−9.0 LUFS**
/// (`0.5` against the `0.1` that gives −23.0 is `20·log10(5) = 13.98 dB` louder), so the stored value must be about
/// that — and the *count* proves it came from the build rather than from a caller.
#[test]
fn a_build_measures_loudness_for_every_file_it_embeds() {
    let scratch = Scratch::new("loudness");
    scratch.file("tone.wav", &wav(48_000, 1000.0, 48_000 * 3));

    let built = build(scratch.path(), stamp(), |_buffer| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.indexed(), 1);
    assert_eq!(built.index.loudness.len(), 1, "the build stored a measurement");
    assert_eq!(built.unmeasured, 0, "and nothing was left unmeasured");

    let hash = *built.index.records.keys().next().expect("one record");
    let measured = built.index.loudness[&hash];
    let integrated = measured.integrated.expect("3 s is seven gating blocks");
    assert!(
        (integrated - -9.0).abs() < 0.5,
        "a 0.5-amplitude tone is ~-9 LUFS (0.1 gives -23.0); got {integrated}"
    );
    assert_eq!(
        measured.blocks, 27,
        "3 s at 400 ms blocks hopping 100 ms is 27 blocks, not the 7 a non-overlapping count gives"
    );
}

/// **A clip too short to gate is stored with an absent integrated loudness**, not with a sentinel — through a real
/// build, not only through `to_text`.
#[test]
fn a_short_file_is_stored_unmeasured_rather_than_at_the_floor() {
    let scratch = Scratch::new("short");
    // 0.2 s: shorter than one 400 ms gating block.
    scratch.file("click.wav", &wav(48_000, 1000.0, 9_600));

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    let hash = *built.index.records.keys().next().expect("one record");
    let measured = built.index.loudness[&hash];
    assert_eq!(measured.integrated, None, "a 0.2 s clip has no integrated loudness");
    assert!(measured.true_peak.is_finite(), "but its peak is a real measurement");
    assert!(measured.too_short() && !measured.silent(), "and the build keeps the two apart");
}

/// **What the summary says when measurement silently produced nothing.** The count exists so a run cannot look
/// complete while storing no loudness at all.
#[test]
fn the_summary_names_unmeasured_files() {
    let built = Built {
        index: Index::new(stamp()),
        failed: Vec::new(),
        not_audio: Vec::new(),
        unattributed: 0,
        sidecar: None,
        unmeasured: 3,
        longer: false,
    };
    assert_eq!(built.describe(), "0 indexed, 3 unmeasured");

    let clean = Built {
        index: Index::new(stamp()),
        failed: Vec::new(),
        not_audio: Vec::new(),
        unattributed: 0,
        sidecar: None,
        unmeasured: 0,
        longer: false,
    };
    assert_eq!(clean.describe(), "0 indexed", "a clean run says nothing extra");
}

/// **A sidecar's licence reaches the asset it names**, which is the join `#14` asked for: a sidecar keys by **file**,
/// the index keys by **content hash**, and this is the only place the two meet.
#[test]
fn a_sidecar_licence_reaches_the_asset_it_names() {
    let scratch = Scratch::new("sidecar");
    scratch.file("step.wav", &wav(48_000, 1000.0, 48_000));
    scratch.file("other.wav", &wav(48_000, 440.0, 48_000));
    scratch.file("licences.csv", b"file,licence,source,author\nstep.wav,CC0-1.0,kenney,Kenney\n");

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.indexed(), 2);
    assert_eq!(built.index.provenance.len(), 1, "only the named file has a record");

    let named = built.index.provenance.values().next().expect("one record");
    assert_eq!(named.licence.as_deref(), Some("CC0-1.0"));
    assert_eq!(named.author.as_deref(), Some("Kenney"));
    assert_eq!(named.permits_commercial_use(), Some(true));

    // **And the unnamed file is absent, not defaulted.** Its result carries `unknown`, which answers `None`.
    let unnamed =
        built.index.records.keys().find(|h| !built.index.provenance.contains_key(h)).expect("the other file");
    assert!(!built.index.provenance.contains_key(unnamed));
    assert_eq!(built.unattributed, 0, "no record went unused");
    assert!(built.sidecar.is_none(), "the sidecar read fine");
}

/// **A sidecar row naming a file the folder does not have is counted**, because that is the commonest typo and silently
/// ignoring it makes a licence look recorded that is attached to nothing.
#[test]
fn a_record_naming_an_absent_file_is_counted() {
    let scratch = Scratch::new("typo");
    scratch.file("step.wav", &wav(48_000, 1000.0, 48_000));
    scratch.file("licences.csv", b"file,licence\nstep.wav,CC0-1.0\nstpe.wav,CC0-1.0\n");

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.index.provenance.len(), 1, "the real file got its licence");
    assert_eq!(built.unattributed, 1, "and the typo is counted, not ignored");
    assert!(built.describe().contains("1 licence records matched no file"), "{}", built.describe());
}

/// **An unreadable sidecar does not silently mean "no licences"** — the two are different facts and the build says
/// which one it found.
#[test]
fn an_unreadable_sidecar_is_reported_while_the_build_still_works() {
    let scratch = Scratch::new("badsidecar");
    scratch.file("step.wav", &wav(48_000, 1000.0, 48_000));
    // A header with no `file` column: the reader refuses it rather than returning an empty map.
    scratch.file("licences.csv", b"licence,source\nCC0-1.0,kenney\n");

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.indexed(), 1, "the audio still indexed");
    assert!(built.index.provenance.is_empty());
    assert!(built.sidecar.is_some(), "and the sidecar failure is carried");
    assert!(built.describe().contains("Licence record unreadable"), "{}", built.describe());
}

/// **A folder with no sidecar is normal**, and says nothing — the distinction the report above depends on.
#[test]
fn no_sidecar_is_normal_and_says_nothing() {
    let scratch = Scratch::new("nosidecar");
    scratch.file("step.wav", &wav(48_000, 1000.0, 48_000));
    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert!(built.sidecar.is_none());
    assert_eq!(built.unattributed, 0);
    assert_eq!(built.describe(), "1 indexed", "a clean run says nothing extra");
}

/// **A licence with a name containing a space survives the join**, end to end — the pack name is the field most likely
/// to contain one, and the format escapes it.
#[test]
fn a_pack_name_with_a_space_survives_a_build() {
    let scratch = Scratch::new("space");
    scratch.file("step.wav", &wav(48_000, 1000.0, 48_000));
    scratch.file("licences.csv", b"file,licence,source\nstep.wav,CC0-1.0,kenney sounds\n");

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    let record = built.index.provenance.values().next().expect("one record");
    assert_eq!(
        record.source,
        Some(bevy_siren::semantic::provenance::Source::Pack(String::from("kenney sounds")))
    );

    // And through the file format too, which is where an unescaped space would split into two fields.
    let back = Index::load(&built.index.to_text(), &stamp()).expect("loads");
    assert_eq!(back.provenance, built.index.provenance);
}

/// **A written index is binary, and `read` still loads one written as text by an older build.**
///
/// The second half is the compatibility promise ADR 0004 makes by choosing "distinguishable by magic, not by
/// extension": an index a project built before this change is a `siren-index.txt` and must keep working.
#[test]
fn a_written_index_is_binary_and_an_old_text_index_still_loads() {
    use bevy_siren::semantic::index::{Index, MAGIC_BINARY};

    let s = Scratch::new("formats");
    s.file("a.wav", &wav(48_000, 440.0, 4800));
    let mut seen = Vec::new();
    let built = build(s.path(), stamp(), recorder(&mut seen)).expect("builds");

    let written = write(s.path(), &built.index).expect("writes");
    let bytes = std::fs::read(&written).expect("read the file");
    assert_eq!(&bytes[..4], MAGIC_BINARY, "the build writes the binary form ADR 0004 chose");
    assert_eq!(read(s.path(), &stamp()).expect("reads back").records, built.index.records);

    // **An old text index still loads** through the same call, because the bytes decide the format.
    std::fs::write(&written, built.index.to_text()).expect("write the text form over it");
    let from_text = read(s.path(), &stamp()).expect("an old text index still loads");
    assert_eq!(from_text.records, built.index.records);
    assert_eq!(from_text.provenance, built.index.provenance);

    // A file that is neither format **says so rather than panicking**, and the message is the most specific one
    // available: bytes that are valid UTF-8 but not an index reach `Index::load`, which names the bad header line
    // instead of the vaguer "not a binary index". A refusal that points at the actual bytes is better than one that
    // only says "wrong".
    std::fs::write(&written, b"not an index at all").expect("write junk");
    let err = read(s.path(), &stamp()).expect_err("junk is refused");
    assert!(err.contains("unreadable index header"), "the specific refusal: {err}");

    // Bytes that are not UTF-8 at all take the other path and say that instead.
    std::fs::write(&written, [0xffu8, 0xfe, 0x00]).expect("write non-UTF-8");
    let err = read(s.path(), &stamp()).expect_err("non-UTF-8 is refused");
    assert!(err.contains("neither a binary index nor UTF-8"), "the other refusal: {err}");

    // `Index` is named here so the import is used even if the assertions above change shape.
    let _: Option<Index> = None;
}

/// **A build now produces tempo and key**, from the same decode as the embedding — the wiring `#24` and `#25` left to
/// this iteration.
///
/// A metronomic click track has a tempo; a short tonal passage has a key. Both must appear in the index without a caller
/// supplying them, which is the difference between a stored field and a produced one.
#[test]
fn a_build_produces_tempo_and_key() {
    // A click every 0.5 s is 120 BPM, held for four seconds so there are enough onsets.
    let mut clicks = vec![0.0f32; 48_000 * 4];
    let period = 24_000; // 0.5 s at 48 kHz
    let mut at = 0;
    while at < clicks.len() {
        for i in 0..240 {
            if at + i >= clicks.len() {
                break;
            }
            let t = i as f32 / 48_000.0;
            clicks[at + i] = (-t * 400.0).exp() * (std::f32::consts::TAU * 1000.0 * t).sin() * 0.6;
        }
        at += period;
    }
    let scratch = Scratch::new("tempo_key");
    scratch.file("click.wav", &wav_from(&clicks));

    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.indexed(), 1);
    assert_eq!(built.index.tempo.len(), 1, "the build produced a tempo");
    assert_eq!(built.index.key.len(), 1, "and a key");

    let hash = *built.index.records.keys().next().expect("one record");
    let tempo = built.index.tempo[&hash];
    let bpm = tempo.bpm.expect("a metronomic click has a tempo");
    assert!((bpm - 120.0).abs() <= 2.0, "0.5 s between clicks is 120 BPM; got {bpm}");
    // A click train has no tonal centre, so the key reports its absence — which is still a *record*.
    assert_eq!(built.index.key[&hash].key.tonic, None, "a click train has no key");
}

/// A clip too short for a chroma window still embeds, and simply has no key record.
#[test]
fn a_clip_too_short_to_analyse_still_embeds() {
    let scratch = Scratch::new("short_analysis");
    // 0.1 s: shorter than key's 8192-sample chroma window.
    scratch.file("tiny.wav", &wav(48_000, 1000.0, 4_800));
    let built = build(scratch.path(), stamp(), |_| Ok(vec![0.5; EMBEDDING_DIM])).expect("build");
    assert_eq!(built.indexed(), 1, "the vector is the index's reason to exist");
    // **0.1 s is 4 800 samples**: shorter than key's 8 192-sample chroma window, so there is no key record — but longer
    // than tempo's two 1 024-sample frames, so tempo *does* record, with `bpm: None`. That distinction is the point of
    // storing the absence rather than omitting the record, and the first version of this test asserted both were
    // missing.
    assert!(built.index.key.is_empty(), "no key record: the clip is shorter than a chroma window");
    let hash = *built.index.records.keys().next().expect("one record");
    let tempo = built.index.tempo.get(&hash).expect("tempo still has enough frames to record");
    assert_eq!(tempo.bpm, None, "but 0.1 s has no rhythm to report");
}

/// A raw f32 buffer as a 16-bit mono WAV at 48 kHz, for a fixture the shared `wav` helper cannot build.
fn wav_from(samples: &[f32]) -> Vec<u8> {
    let rate = 48_000u32;
    let data_len = (samples.len() * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        v.extend_from_slice(&((s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16).to_le_bytes());
    }
    v
}

/// **`#36`: the decode path is at or above its stated target**, so the constant cannot drift from the code.
///
/// This measures the crate's **own** work — the analysis of a decoded buffer, which is what `indexbench`'s `decode`
/// column reports — rather than a model call, because a model is not available here and a synthetic stand-in would be a
/// number about nothing.
///
/// Release-only for the same reason as the query budget: the arithmetic is 19x slower in a debug build, and asserting a
/// released figure against it would teach a reader to ignore this test.
#[cfg(feature = "semantic")]
#[test]
fn the_decode_path_meets_its_throughput_target() {
    use bevy_siren::semantic::indexer::INDEX_TARGET_CLIPS_PER_SEC;

    if cfg!(debug_assertions) {
        println!(
            "skipped: a released-build figure; run `cargo test --release --features semantic` to enforce it"
        );
        return;
    }

    // **The four analyses a build runs per clip**, on the clip length `indexbench` generates — **0.5 s**, which is what
    // makes the two figures comparable. A 4 s buffer measures 83 clips/s rather than ~750, because the analysis work is
    // proportional to duration while a "clip" is not: the target is per clip of a stated length, and that is stated in
    // the constant's doc.
    let rate = 48_000usize;
    let mut buffer = vec![0.0f32; rate / 2];
    let mut at = 0;
    while at < buffer.len() {
        for i in 0..240 {
            if at + i < buffer.len() {
                let t = i as f32 / rate as f32;
                buffer[at + i] = (-t * 400.0).exp() * (std::f32::consts::TAU * 1000.0 * t).sin() * 0.6;
            }
        }
        at += rate / 2;
    }

    let clips = 400;
    let start = std::time::Instant::now();
    for _ in 0..clips {
        let _ = bevy_siren::semantic::loudness::measure(&buffer, rate as u32);
        let spectra = bevy_siren::semantic::analysis::spectra(&buffer);
        let _ = bevy_siren::semantic::spectral::analyse_spectra(&buffer, &spectra);
        let _ = bevy_siren::semantic::tempo::estimate_spectra(&buffer, &spectra);
        let _ = bevy_siren::semantic::key::estimate(&buffer);
    }
    let elapsed = start.elapsed().as_secs_f64();
    let clips_per_second = clips as f64 / elapsed;

    assert!(
        clips_per_second >= INDEX_TARGET_CLIPS_PER_SEC,
        "the analysis ran at {clips_per_second:.0} clips/s, below the {INDEX_TARGET_CLIPS_PER_SEC} target: either the \
         analysis regressed or the target needs re-measuring"
    );
}
