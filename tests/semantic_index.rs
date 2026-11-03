//! The index: a round trip that is bit-identical, and three refusals that must fire.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::index::{Index, IndexProblem, ModelStamp};
use bevy_siren::semantic::{EMBEDDING_DIM, MODEL_SAMPLE_RATE};

fn stamp() -> ModelStamp {
    ModelStamp::measured_clap()
}

fn vector(seed: f32) -> Vec<f32> {
    (0..EMBEDDING_DIM).map(|i| (seed + i as f32) * 0.001).collect()
}

fn index_with(n: u64) -> Index {
    let mut index = Index::new(stamp());
    for h in 0..n {
        index.insert(0x1000 + h, vector(h as f32)).expect("a full-length vector");
    }
    index
}

#[test]
fn a_round_trip_is_bit_identical() {
    let original = index_with(3);
    let text = original.to_text();
    let back = Index::load(&text, &stamp()).expect("its own file loads");
    assert_eq!(back.stamp, original.stamp, "the stamp survives");
    assert_eq!(back.records.len(), original.records.len());
    for (hash, v) in &original.records {
        assert_eq!(back.records.get(hash), Some(v), "vector {hash:#x} is bit-identical, not merely close");
    }
}

#[test]
fn a_float_round_trips_through_its_shortest_form() {
    // The claim the format depends on: `{:?}` on an f32 parses back to the same bits. If this were false, every
    // round-trip test above would be hiding a tolerance.
    let awkward: Vec<f32> = vec![0.1, 1.0 / 3.0, f32::MIN_POSITIVE, -1.234_567_9e-7, f32::MAX];
    for v in awkward {
        let text = format!("{v:?}");
        let back: f32 = text.parse().expect("parses");
        assert_eq!(back.to_bits(), v.to_bits(), "{text} round-trips exactly");
    }
}

#[test]
fn a_model_mismatch_is_refused_naming_both_versions() {
    let text = index_with(1).to_text();
    let mut loaded = stamp();
    loaded.version = "a-different-model".to_string();

    let err = Index::load(&text, &loaded).expect_err("a mismatched index must not load");
    match &err {
        IndexProblem::ModelMismatch(m) => {
            assert_eq!(m.index.version, stamp().version);
            assert_eq!(m.loaded.version, "a-different-model");
        }
        other => panic!("expected a model mismatch, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(msg.contains("Re-index"), "the message says what to do: {msg}");
    assert!(msg.contains("clap-htsat-fused"), "and names the model: {msg}");
}

#[test]
fn a_truncated_index_is_detected_rather_than_read_as_partial() {
    let full = index_with(3).to_text();
    // Cut the last record: the body is now shorter than the header claims.
    let cut = match full.rfind('\n') {
        Some(i) => &full[..i + 1],
        None => &full[..],
    };
    let truncated = cut[..cut.trim_end_matches('\n').rfind('\n').expect("two records") + 1].to_string();

    let err = Index::load(&truncated, &stamp()).expect_err("a truncated file must not load");
    // Either the checksum or the record count catches it, and both are honest detections. What must never happen is a
    // silent partial load.
    assert!(
        matches!(err, IndexProblem::Truncated { .. } | IndexProblem::Checksum { .. }),
        "truncation is detected as truncation or corruption, not accepted: {err:?}"
    );
}

#[test]
fn a_corrupted_byte_is_detected() {
    let mut text = index_with(2).to_text();
    // Flip a digit inside the body, leaving the header's checksum stale.
    let body_start = text.match_indices('\n').nth(6).map(|(i, _)| i + 1).expect("seven header lines");
    let at = body_start + 3;
    let original = text.as_bytes()[at];
    text.replace_range(at..at + 1, if original == b'0' { "1" } else { "0" });

    let err = Index::load(&text, &stamp()).expect_err("a corrupt body must not load");
    assert!(matches!(err, IndexProblem::Checksum { .. }), "got {err:?}");
    assert!(err.to_string().contains("corrupt"), "{err}");
}

#[test]
fn a_vector_of_the_wrong_length_cannot_enter() {
    let mut index = Index::new(stamp());
    let err = index.insert(1, vec![0.0; EMBEDDING_DIM - 1]).expect_err("a short vector is refused");
    assert!(matches!(err, IndexProblem::Record { .. }));
    assert!(err.to_string().contains(&format!("{}", EMBEDDING_DIM - 1)), "{err}");
    assert!(index.is_empty(), "and nothing was inserted");
}

#[test]
fn an_empty_index_round_trips() {
    let empty = Index::new(stamp());
    let text = empty.to_text();
    let back = Index::load(&text, &stamp()).expect("an empty index is a valid index");
    assert!(back.is_empty());
    assert_eq!(back.stamp, empty.stamp);
}

#[test]
fn the_stamp_names_the_sample_rate_and_dimension_the_model_reported() {
    let s = stamp();
    assert_eq!(s.sample_rate, MODEL_SAMPLE_RATE);
    assert_eq!(s.dimension, EMBEDDING_DIM);
    assert_eq!(s.model, "laion/clap-htsat-fused");
    assert!(s.to_string().contains("48000Hz"), "a stamp is readable: {s}");
}

#[test]
fn a_file_that_is_not_an_index_says_so() {
    let err = Index::load("hello, this is a text file\n", &stamp()).expect_err("not an index");
    assert!(matches!(err, IndexProblem::Header { .. }), "got {err:?}");
    let err = Index::load("", &stamp()).expect_err("an empty file is not an index");
    assert!(matches!(err, IndexProblem::Header { .. }), "got {err:?}");
}
