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

/// **The provenance section round-trips, and an index with none is byte-identical to the old format.**
///
/// The second half is the additive promise: `to_text` for an index with no provenance must equal what the crate wrote
/// before the section existed, or every consumer re-writes its index on upgrade for no reason.
#[test]
fn provenance_round_trips_and_is_absent_from_an_index_with_none() {
    use bevy_siren::semantic::provenance::{Provenance, Source};

    let mut index = index_with(3);
    index.provenance.insert(
        1,
        Provenance {
            licence: Some(String::from("CC0-1.0")),
            source: Some(Source::Pack(String::from("kenney sounds"))), // a space, which the format must escape
            author: Some(String::from("Kenney")),
        },
    );
    index.provenance.insert(2, Provenance { licence: None, source: Some(Source::Unknown), author: None });
    let text = index.to_text();
    let back = Index::load(&text, &stamp()).expect("loads");
    assert_eq!(back.provenance, index.provenance, "every field survives, including the escaped space");
    assert_eq!(back.provenance[&1].licence.as_deref(), Some("CC0-1.0"));
    assert_eq!(
        back.provenance[&1].source,
        Some(Source::Pack(String::from("kenney sounds"))),
        "the space inside the pack name is not a field separator"
    );
    assert_eq!(back.provenance[&2].source, Some(Source::Unknown), "recorded-unknown differs from absent");
    assert_eq!(back.provenance[&2].licence, None, "and an absent licence stays absent");

    // **The additive promise.** An index with no provenance writes no section at all.
    let plain = index_with(3).to_text();
    assert!(!plain.contains("provenance "), "an index with no provenance carries no section: {plain}");
    let plain_back = Index::load(&plain, &stamp()).expect("loads");
    assert!(plain_back.provenance.is_empty());
}

/// **A recorded empty licence is distinguishable from an absent one after a round trip** — `#14`'s first criterion,
/// carried all the way through the file format rather than only in memory.
#[test]
fn an_empty_licence_survives_the_file_distinguishably_from_an_absent_one() {
    use bevy_siren::semantic::provenance::Provenance;

    let mut index = index_with(2);
    index.provenance.insert(1, Provenance { licence: Some(String::new()), ..Provenance::unknown() });
    index.provenance.insert(2, Provenance::unknown());
    let back = Index::load(&index.to_text(), &stamp()).expect("loads");

    assert_eq!(back.provenance[&1].licence.as_deref(), Some(""), "recorded as empty");
    assert_eq!(back.provenance[&2].licence, None, "not recorded");
    assert!(back.provenance[&1].is_recorded());
    assert!(!back.provenance[&2].is_recorded());
}

/// **A truncated provenance section is refused**, the same way a truncated vector body is.
#[test]
fn a_provenance_section_promising_more_records_than_it_holds_is_refused() {
    let mut index = index_with(1);
    index.provenance.insert(1, bevy_siren::semantic::provenance::Provenance::unknown());
    let text = index.to_text();
    // Claim two records, keep one, and re-checksum so the count is the only thing wrong.
    let body_start = text.match_indices('\n').nth(6).map(|(i, _)| i + 1).expect("header");
    let (header, body) = text.split_at(body_start);
    let broken_body = body.replace("provenance 1", "provenance 2");
    let checksum = bevy_siren::library::fnv1a(broken_body.as_bytes());
    let mut fixed_header: Vec<&str> = header.lines().collect();
    let last = fixed_header.len() - 1;
    let replaced = format!("checksum {checksum:016x}");
    fixed_header[last] = &replaced;
    let broken = format!("{}\n{broken_body}", fixed_header.join("\n"));
    assert!(
        Index::load(&broken, &stamp()).is_err(),
        "a section that promises two records and carries one is truncated, not valid"
    );
}

/// **`#14` criterion 3: a search result reports the asset's licence and source** — carried on the result, not fetched
/// separately, so a consumer cannot forget to ask.
#[test]
fn a_search_result_carries_its_provenance() {
    use bevy_siren::semantic::provenance::{Provenance, Source};
    use bevy_siren::semantic::rank::top_k;

    // A fresh index rather than `index_with`, so the two hashes below are the ONLY ones — otherwise the ranking
    // returns the fixture's vectors too and the assertion is about the wrong result.
    let mut index = Index::new(stamp());
    index.records.insert(1, {
        let mut v = vec![0.0f32; 512];
        v[0] = 1.0;
        v
    });
    index.records.insert(2, {
        let mut v = vec![0.0f32; 512];
        v[1] = 1.0;
        v
    });
    index.provenance.insert(
        1,
        Provenance {
            licence: Some(String::from("CC0-1.0")),
            source: Some(Source::Pack(String::from("kenney"))),
            author: Some(String::from("Kenney")),
        },
    );

    let mut query = vec![0.0f32; 512];
    query[0] = 1.0;
    let hits = top_k(&index, &query, 2).expect("ranked");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].hash, 1);
    assert_eq!(hits[0].provenance.licence.as_deref(), Some("CC0-1.0"), "the result reports its licence");
    assert_eq!(hits[0].provenance.source, Some(Source::Pack(String::from("kenney"))));

    // **An asset with no record still carries a value**, so a caller printing a result always has something to print.
    assert_eq!(hits[1].hash, 2);
    assert_eq!(hits[1].provenance, Provenance::unknown());
    assert_eq!(hits[1].provenance.permits_commercial_use(), None, "and it never reads as permissive");
}

/// **The never-forget property**: a result cannot exist without a provenance value, so no caller can present an
/// unlicensed asset as if it had no obligation.
#[test]
fn a_result_cannot_be_built_without_provenance() {
    // This is a compile-time property, so the test is that the field is not optional: any `Match` has one.
    let index = index_with(1);
    let mut query = vec![0.0f32; 512];
    query[0] = 1.0;
    let hits = bevy_siren::semantic::rank::top_k(&index, &query, 1).expect("ranked");
    assert!(!hits[0].provenance.is_recorded(), "defaults to the explicit unknown");
}

/// **A loudness section round-trips, and a short clip's absent loudness survives the file as absence.**
///
/// This is the whole reason the section is not a numeric column: `Loudness::integrated` is `None` for a 0.2 s clip, and
/// ADR 0003 measured that both `ffmpeg` (`-70.0`) and `ebur128` (`-inf`) report a *number* there — one of which is
/// also a real level. Writing `-` keeps "too short to measure" distinguishable from "measured at −70".
#[test]
fn loudness_round_trips_and_a_short_clip_stays_absent() {
    use bevy_siren::semantic::loudness::Loudness;

    let mut index = index_with(2);
    let quiet = Loudness {
        integrated: Some(-23.01),
        true_peak: -20.0,
        sample_peak: -20.0,
        blocks: 7,
        blocks_possible: 7,
    };
    let short = Loudness {
        integrated: None, // a 0.2 s clip: no gating block, so no measurement
        true_peak: -6.02,
        sample_peak: -6.02,
        blocks: 0,
        blocks_possible: 0,
    };
    index.loudness.insert(0x1000, quiet);
    index.loudness.insert(0x1001, short);

    let back = Index::load(&index.to_text(), &stamp()).expect("loads");
    assert_eq!(back.loudness, index.loudness, "every field survives, including the absent loudness");

    let a = back.loudness[&0x1000];
    assert_eq!(a.integrated, Some(-23.01));
    assert_eq!(a.blocks, 7);

    let b = back.loudness[&0x1001];
    assert_eq!(b.integrated, None, "a short clip has NO loudness after the round trip, not a sentinel");
    assert_eq!(b.true_peak, -6.02, "but its peak is still a real measurement");
    assert!(b.too_short());
    assert!(!b.silent(), "a short loud clip is not silence, and the file keeps that distinction");
}

/// **An index with no loudness writes no section**, so every file written before this existed is byte-identical.
#[test]
fn an_index_with_no_loudness_writes_no_section() {
    let plain = index_with(2).to_text();
    assert!(!plain.contains("loudness "), "no section when there is nothing to write");
    assert!(Index::load(&plain, &stamp()).expect("loads").loudness.is_empty());
}

/// **Both sections coexist and their order does not matter.**
#[test]
fn provenance_and_loudness_sections_coexist() {
    use bevy_siren::semantic::loudness::Loudness;
    use bevy_siren::semantic::provenance::{Provenance, Source};

    let mut index = index_with(1);
    index.provenance.insert(
        0x1000,
        Provenance {
            licence: Some(String::from("CC0-1.0")),
            source: Some(Source::Pack(String::from("k"))),
            author: None,
        },
    );
    index.loudness.insert(
        0x1000,
        Loudness {
            integrated: Some(-14.0),
            true_peak: -1.0,
            sample_peak: -1.0,
            blocks: 9,
            blocks_possible: 9,
        },
    );
    let back = Index::load(&index.to_text(), &stamp()).expect("loads");
    assert_eq!(back.provenance[&0x1000].licence.as_deref(), Some("CC0-1.0"));
    assert_eq!(back.loudness[&0x1000].integrated, Some(-14.0));
}

/// **A non-finite level is written as absence, not as `NaN` in a text field.** The crate's rule is that a number which
/// is not a number is silence, and a file that could not round-trip it must not pretend it did.
#[test]
fn a_non_finite_level_makes_the_record_unrepresentable() {
    use bevy_siren::semantic::loudness::Loudness;

    let mut index = index_with(1);
    index.loudness.insert(
        0x1000,
        Loudness {
            integrated: Some(f32::NAN),
            true_peak: f32::INFINITY,
            sample_peak: 0.0,
            blocks: 1,
            blocks_possible: 1,
        },
    );
    // `db_field` writes `-` for a non-finite level, and `parse_loudness_line` requires a number in that column, so a
    // record with an unusable peak cannot round-trip. The parser **refuses the file by name** rather than dropping the
    // record or substituting `0.0` — the latter would look like a measured silence, which is the class of lie this
    // section exists to prevent. A refusal is loud; a substitution is not.
    let err = Index::load(&index.to_text(), &stamp()).expect_err("a non-finite level cannot round-trip");
    match err {
        IndexProblem::Record { what, .. } => {
            assert!(what.contains("loudness record"), "the refusal names the section: {what}")
        }
        other => panic!("{other:?}"),
    }
}
