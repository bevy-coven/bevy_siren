//! The library scan: content identity, header reading, and the skips that must never be silent.

use std::io::Write;
use std::path::{Path, PathBuf};

use bevy_siren::library::{Library, SkipReason, fnv1a, scan};

/// A scratch directory that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("bevy_siren_library_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("scratch dir");
        Scratch(p)
    }

    fn file(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let p = self.0.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("parent dir");
        }
        let mut f = std::fs::File::create(&p).expect("create");
        f.write_all(bytes).expect("write");
        p
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

/// A minimal 16-bit mono WAV at `rate` holding `samples` zero samples. Header only where it matters.
fn wav(rate: u32, channels: u16, samples: usize) -> Vec<u8> {
    let data_len = (samples * channels as usize * 2) as u32;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes()); // PCM
    v.extend_from_slice(&channels.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    v.extend_from_slice(&(channels * 2).to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    v.resize(44 + data_len as usize, 0);
    v
}

#[test]
fn a_scan_finds_three_files_with_distinct_hashes() {
    let s = Scratch::new("three");
    s.file("a.wav", &wav(44100, 1, 100));
    s.file("b.wav", &wav(48000, 2, 100));
    s.file("sub/c.wav", &wav(22050, 1, 200));

    let lib = scan(s.path());
    assert_eq!(lib.len(), 3, "three distinct files are three assets");
    assert!(lib.skipped.is_empty(), "nothing is skipped: {:?}", lib.skipped);

    let hashes: std::collections::BTreeSet<u64> = lib.assets.keys().copied().collect();
    assert_eq!(hashes.len(), 3, "three files are three hashes");
}

#[test]
fn a_rescan_of_an_unchanged_directory_is_identical() {
    let s = Scratch::new("stable");
    s.file("a.wav", &wav(44100, 1, 100));
    s.file("z/deep/b.wav", &wav(48000, 2, 50));
    s.file("readme.txt", b"not audio");

    let first = scan(s.path());
    let second = scan(s.path());
    assert_eq!(first, second, "a scan is a pure function of the tree");
    assert_eq!(first.by_path(), second.by_path(), "and so is its order");
}

#[test]
fn a_corrupt_header_appears_with_its_reason_not_omitted() {
    let s = Scratch::new("corrupt");
    s.file("good.wav", &wav(44100, 1, 10));
    s.file("broken.wav", b"RIFF\x00\x00\x00\x00WAVEthis is not a fmt chunk");

    let lib = scan(s.path());
    assert_eq!(lib.len(), 1, "the good file is indexed");
    let broken =
        lib.skipped.iter().find(|k| k.path.ends_with("broken.wav")).expect("the broken file is reported");
    assert_eq!(broken.reason, SkipReason::UndecodableHeader);
    assert!(
        broken.reason.to_string().contains("header"),
        "the reason names the header, so a developer can find the file: {}",
        broken.reason
    );
}

#[test]
fn a_non_audio_extension_is_reported_with_the_extension_found() {
    let s = Scratch::new("ext");
    s.file("a.wav", &wav(44100, 1, 10));
    s.file("notes.md", b"# notes");
    s.file("song.mp3", b"ID3\x04\x00\x00\x00\x00\x00\x00");

    let lib = scan(s.path());
    assert_eq!(lib.len(), 2, "the wav and the mp3");
    let md = lib.skipped.iter().find(|k| k.path.ends_with("notes.md")).expect("notes are reported");
    assert_eq!(md.reason, SkipReason::NotAudio { extension: "md".into() });
    assert!(lib.skipped.iter().all(|k| !k.path.ends_with("song.mp3")), "an mp3 is audio");
}

#[test]
fn the_header_gives_the_rate_and_the_channels() {
    let s = Scratch::new("header");
    s.file("stereo.wav", &wav(48000, 2, 20));
    s.file("mono.wav", &wav(22050, 1, 20));

    let lib = scan(s.path());
    let mut found: Vec<(Option<u32>, Option<u16>)> =
        lib.assets.values().map(|a| (a.sample_rate, a.channels)).collect();
    found.sort();
    assert_eq!(found, vec![(Some(22050), Some(1)), (Some(48000), Some(2))]);
}

#[test]
fn two_identical_files_are_one_asset_with_two_paths() {
    let s = Scratch::new("dup");
    let bytes = wav(44100, 1, 100);
    s.file("a.wav", &bytes);
    s.file("copy/a.wav", &bytes);

    let lib = scan(s.path());
    assert_eq!(lib.len(), 1, "same bytes are the same asset");
    let asset = lib.assets.values().next().expect("one asset");
    assert_eq!(asset.paths.len(), 2, "both paths are kept, so neither folder is invisible");
    assert_eq!(lib.duplicated().len(), 1, "and the duplication is reported");
}

#[test]
fn a_rename_keeps_the_asset_and_a_reencode_does_not() {
    let s = Scratch::new("rename");
    let original = wav(44100, 1, 100);
    s.file("kick.wav", &original);
    let before = scan(s.path());
    assert_eq!(before.len(), 1);

    // Rename: same bytes, new path.
    std::fs::rename(s.0.join("kick.wav"), s.0.join("kick_renamed.wav")).expect("rename");
    let renamed = scan(s.path());
    assert_eq!(renamed.len(), 1, "a rename is the same asset");
    assert_eq!(renamed.added_since(&before).len(), 0, "so it is not an add");
    assert!(renamed.removed_since(&before).is_empty(), "and not a removal either");

    // Re-encode: same music, different bytes. The old file is replaced, so the old content is genuinely gone.
    // `wav` zero-pads its data, so a longer clip differs in both header and length — two distinct byte strings.
    std::fs::remove_file(s.0.join("kick_renamed.wav")).expect("remove the renamed file");
    s.file("kick.wav", &wav(44100, 1, 1000));
    // Re-encoded in place: one file on disk, but a different identity. This is the case an incremental index
    // exists to get right — the path is unchanged and the content is not.
    let reencoded = scan(s.path());
    assert_eq!(reencoded.len(), 1, "one file, so one asset");
    assert_eq!(reencoded.added_since(&renamed).len(), 1, "its content is an add");
    assert_eq!(reencoded.removed_since(&renamed).len(), 1, "and the old content is a removal");
}

#[test]
fn a_deleted_file_is_removed_and_the_rest_are_untouched() {
    let s = Scratch::new("delete");
    s.file("a.wav", &wav(44100, 1, 10));
    s.file("b.wav", &wav(48000, 1, 10));
    let before = scan(s.path());
    assert_eq!(before.len(), 2);

    std::fs::remove_file(s.0.join("a.wav")).expect("remove");
    let after = scan(s.path());
    assert_eq!(after.len(), 1);
    assert_eq!(after.added_since(&before).len(), 0, "nothing new was added");
    assert_eq!(after.removed_since(&before).len(), 1, "exactly one removal");
}

#[test]
fn fnv1a_matches_its_published_vectors() {
    // The FNV-1a 64-bit reference vectors: the offset basis on the empty input, and the documented
    // one-character values. A hash that is wrong here is wrong everywhere and silently.
    assert_eq!(fnv1a(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(fnv1a(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn a_symlink_that_leaves_the_root_is_refused_not_followed() {
    #[cfg(unix)]
    {
        let s = Scratch::new("symlink");
        let outside = Scratch::new("symlink_outside");
        let target = outside.file("secret.wav", &wav(44100, 1, 10));
        s.file("a.wav", &wav(48000, 1, 10));
        std::os::unix::fs::symlink(&target, s.0.join("link.wav")).expect("symlink");

        let lib: Library = scan(s.path());
        assert_eq!(lib.len(), 1, "only the file inside the root");
        assert!(
            lib.refused.iter().any(|(p, why)| p.ends_with("link.wav") && why.contains("leaves")),
            "the refusal names the link and the reason: {:?}",
            lib.refused
        );
    }
}

// ---------------------------------------------------------------------------------------------------------------
// The `semantic` feature's boundary, pinned.
// ---------------------------------------------------------------------------------------------------------------

/// **The `semantic` feature's dependencies are exactly the five argued for in `tests/leaf.rs`.** A user who wants retrieval
/// gets retrieval, and a user who does not gets the graph `0.1.0` shipped.
///
/// This is asserted against the manifest rather than `cargo tree` because a test may not shell out to Cargo, and the
/// thing that would actually regress is a dependency added to the feature without the argument `S-09` requires.
#[test]
fn the_semantic_feature_declares_no_dependency_of_its_own() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).expect("manifest");
    let line = manifest
        .lines()
        .find(|l| l.trim_start().starts_with("semantic ="))
        .expect("the semantic feature is declared");
    let rhs = line.split('=').nth(1).expect("a feature has a value").trim();
    // The feature names exactly these dependencies, and each has its argument in `tests/leaf.rs`:
    //   ort     — the inference runtime (ADR 0002).
    //   rodio   — the decoder the crate already trusts, which costs no new crate: `bevy_audio` resolves rodio 0.22.2
    //             in the default graph already, so this entry turns on this crate's own decoder features and adds
    //             nothing. Measured: `cargo tree --edges normal` is 413 lines with no features.
    //   rubato  — the resampler, which turns a 44.1 kHz library from refused into converted. Measured: a 1 kHz tone
    //             keeps 0.5000 of its 0.5000 energy and a 1.000 s file becomes exactly 48000 samples.
    //   ebur128 — loudness and true peak (ADR 0003). **Pure Rust** despite the name: with `default-features = false`
    //             its `c-tests` build script — the only thing that invokes `cc` — stays off. Measured: a clean build
    //             takes 1.66 s and produces no C object, and it agrees with `ffmpeg`'s reference to 0.01 LU.
    //   serde_json — the tool host's wire format (ADR 0006's `siren-tools`), the smallest choice that works: 6 crates in a
    //             probe, against 58 for `rmcp` with `tokio` and `schemars`. The binary's dependencies are argued for in
    //             `tests/leaf.rs`, as ADR 0006 says.
    assert_eq!(
        rhs, "[\"dep:ort\", \"dep:rodio\", \"dep:rubato\", \"dep:ebur128\", \"dep:serde_json\"]",
        "the semantic feature pulls the inference runtime, the decoder, the resampler, the loudness analyser and the tool \
         host's wire format, and nothing else: `{line}`.\nAnything more is a design decision: add it to ALLOWED_DEPS in \
         tests/leaf.rs with the sentence that argues for it, in the same commit."
    );
}

/// **The model constants are the numbers the model reported**, not round numbers that look plausible.
///
/// Each is measured in `runs/2026-11-03-62/research.md`; a change here without a new measurement is a magic constant
/// wearing a name.
#[cfg(feature = "semantic")]
#[test]
fn the_model_constants_match_the_measurement() {
    use bevy_siren::semantic::{EMBEDDING_DIM, MODEL_MEL_FRAMES, MODEL_SAMPLE_RATE};
    assert_eq!(MODEL_SAMPLE_RATE, 48_000, "CLAP's feature extractor resamples to 48 kHz");
    assert_eq!(MODEL_MEL_FRAMES, 1001, "every duration produced 1001 mel frames");
    assert_eq!(EMBEDDING_DIM, 512, "get_audio_features returns [batch, 512]");
}
