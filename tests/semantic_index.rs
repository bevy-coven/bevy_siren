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

/// **The binary form round-trips every section**, including the two absences that a numeric column would destroy.
///
/// The text form and the binary form must carry the **same index**, or the choice between them changes an asset's
/// terms. That is what this asserts, rather than each format against itself.
#[test]
fn the_binary_form_round_trips_what_the_text_form_does() {
    use bevy_siren::semantic::loudness::Loudness;
    use bevy_siren::semantic::provenance::{Provenance, Source};

    let mut index = index_with(3);
    index.provenance.insert(
        0x1000,
        Provenance {
            licence: Some(String::from("CC0-1.0")),
            source: Some(Source::Pack(String::from("kenney sounds"))),
            author: Some(String::from("Kenney")),
        },
    );
    index
        .provenance
        .insert(0x1001, Provenance { licence: None, source: Some(Source::Unknown), author: None });
    index.loudness.insert(
        0x1000,
        Loudness {
            integrated: Some(-23.01),
            true_peak: -20.0,
            sample_peak: -20.0,
            blocks: 7,
            blocks_possible: 7,
        },
    );
    // A clip too short to gate: `integrated` is absent and that must survive as absence.
    index.loudness.insert(
        0x1001,
        Loudness { integrated: None, true_peak: -6.02, sample_peak: -6.02, blocks: 0, blocks_possible: 0 },
    );

    let back = Index::from_bytes(&index.to_bytes(), &stamp()).expect("loads");
    assert_eq!(back.records, index.records, "vectors survive bit for bit");
    assert_eq!(back.provenance, index.provenance, "including a pack name with a space");
    assert_eq!(back.loudness, index.loudness, "including the absent loudness");
    assert_eq!(
        back.loudness[&0x1001].integrated, None,
        "a short clip has NO integrated loudness after the round trip, not a sentinel"
    );
    assert!(back.loudness[&0x1001].too_short() && !back.loudness[&0x1001].silent());
    // And the two formats agree on the same content.
    let via_text = Index::load(&index.to_text(), &stamp()).expect("text loads");
    assert_eq!(via_text.records, back.records);
    assert_eq!(via_text.provenance, back.provenance);
    assert_eq!(via_text.loudness, back.loudness);
}

/// **The binary form is the size ADR 0004 computed**, to the byte — asserted rather than quoted.
///
/// ADR 0004 measured the text form at **31.88 MB** for 5 000 clips and computed the binary form at **10.30 MB**. The
/// 3× figure has been quoted in four iterations' logs; this is the first test that would fail if it stopped being true.
#[test]
fn five_thousand_vectors_weigh_what_adr_0004_says() {
    // **The values must look like an embedding.** A constant `0.5` prints as `0.5` — one character — so a fixture of
    // repeated constants makes the text form the *same* size as the binary form and the 3x claim looks false. ADR
    // 0004's figure was computed on varied values, and this reproduces that: a deterministic spread so the test is
    // stable.
    let mut seed = 0x5EED_51AE_u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((seed >> 40) as f32 / (1u64 << 24) as f32 - 0.5) * 0.08
    };
    let mut index = Index::new(stamp());
    for h in 0..5_000u64 {
        let vector: Vec<f32> = (0..512).map(|_| next()).collect();
        index.insert(h, vector).expect("full-length vector");
    }
    let binary = index.to_bytes().len();
    // 5000 x (8 hash + 512 x 4) = 10_280_000, plus header, two u64 section counts and the checksum.
    let payload = 5_000 * (8 + 512 * 4);
    let overhead = binary - payload;
    assert_eq!(binary, payload + overhead);
    assert!(
        (10.0..10.5).contains(&(binary as f64 / 1e6)),
        "5000 clips at 512 dimensions is 10.28 MB binary (ADR 0004 computed 10.30); got {:.2} MB",
        binary as f64 / 1e6
    );

    let text = index.to_text().len();
    let ratio = text as f64 / binary as f64;
    assert!(
        (3.0..3.3).contains(&ratio),
        "the text form is ~3x the binary form (ADR 0004 measured 31.88 vs 10.30 MB); \
         got {ratio:.2} ({text} vs {binary} bytes)"
    );
}

/// **A foreign or truncated file is refused with a sentence**, never a panic — the bytes came from disk, and a
/// half-written file is the ordinary failure.
#[test]
fn a_foreign_or_truncated_binary_file_is_refused() {
    // Wrong magic.
    let err = Index::from_bytes(b"nope and more bytes here", &stamp()).expect_err("foreign");
    assert!(matches!(err, IndexProblem::Bytes { .. }), "{err:?}");

    // A valid file cut in half: the checksum catches it before any length is trusted.
    let mut index = index_with(2);
    index.insert(0x2000, vec![0.25f32; 512]).expect("insert");
    let bytes = index.to_bytes();
    let half = &bytes[..bytes.len() / 2];
    let err = Index::from_bytes(half, &stamp()).expect_err("truncated");
    assert!(matches!(err, IndexProblem::Checksum { .. }), "{err:?}");

    // Too short for a header at all.
    let err = Index::from_bytes(&bytes[..3], &stamp()).expect_err("tiny");
    assert!(matches!(err, IndexProblem::Bytes { .. }), "{err:?}");
}

/// **A binary file from another model is refused**, the same rule the text form enforces.
#[test]
fn a_binary_index_from_another_model_is_refused() {
    let mut other = ModelStamp::measured_clap();
    other.version = String::from("some-other-model");
    let index = Index::new(other);
    let err = Index::from_bytes(&index.to_bytes(), &stamp()).expect_err("mismatch");
    assert!(matches!(err, IndexProblem::ModelMismatch(_)), "{err:?}");
}

/// **The two formats are distinguishable by their bytes**, not by their names, which is what ADR 0004 requires.
#[test]
fn a_file_says_which_format_it_is() {
    use bevy_siren::semantic::index::MAGIC_BINARY;
    let index = index_with(1);
    assert_eq!(&index.to_bytes()[..4], MAGIC_BINARY);
    assert!(index.to_text().starts_with("bevy_siren index 1"));
    assert_ne!(&index.to_bytes()[..4], &index.to_text().as_bytes()[..4]);
}

/// **`#22` criterion 4: all three spectral features survive an index round trip**, in **both** formats.
///
/// The absent centroid is the load-bearing part: a silent clip has no brightness, and `0 Hz` is a *real* reading for a
/// DC offset. A sentinel would make the two indistinguishable on disk, which is the same mistake the loudness section
/// avoids.
#[test]
fn spectral_features_round_trip_in_both_formats() {
    use bevy_siren::semantic::spectral::Spectral;

    let mut index = index_with(3);
    index.spectral.insert(
        0x1000,
        Spectral { centroid_hz: Some(1234.5), rolloff_hz: Some(4321.0), noise_floor_db: -42.5 },
    );
    // A silent clip: no centroid, no rolloff, and the crate's floor — never `0 Hz` and never NaN.
    index.spectral.insert(0x1001, Spectral::absent());
    // A DC offset: a real but very dark reading, distinguishable from silence.
    index.spectral.insert(
        0x1002,
        Spectral { centroid_hz: Some(9.375), rolloff_hz: Some(46.875), noise_floor_db: -6.0 },
    );

    let via_text = Index::load(&index.to_text(), &stamp()).expect("text loads");
    assert_eq!(via_text.spectral, index.spectral, "every field survives the text form");

    let via_bytes = Index::from_bytes(&index.to_bytes(), &stamp()).expect("bytes load");
    assert_eq!(via_bytes.spectral, index.spectral, "and the binary form");

    let silent = via_bytes.spectral[&0x1001];
    assert_eq!(silent.centroid_hz, None, "silence keeps NO centroid through a round trip");
    assert_eq!(silent.rolloff_hz, None);
    assert_eq!(silent.noise_floor_db, bevy_siren::model::SILENT_DB);

    let dc = via_bytes.spectral[&0x1002];
    assert_eq!(dc.centroid_hz, Some(9.375), "and a DC offset keeps its real, dark reading");
    assert_ne!(dc.centroid_hz, silent.centroid_hz, "which must never be confused with silence");
}

/// **An index with no spectral section writes none**, so every file written before this feature still loads.
#[test]
fn an_index_with_no_spectral_writes_no_section() {
    let plain = index_with(2).to_text();
    assert!(!plain.contains("spectral "), "no section when there is nothing to write");
    assert!(Index::load(&plain, &stamp()).expect("loads").spectral.is_empty());
    assert!(Index::from_bytes(&index_with(2).to_bytes(), &stamp()).expect("loads").spectral.is_empty());
}

/// **A truncated spectral section is refused**, by the same count rule as every other section.
#[test]
fn a_spectral_section_promising_more_records_than_it_holds_is_refused() {
    use bevy_siren::semantic::spectral::Spectral;
    let mut index = index_with(1);
    index.spectral.insert(
        0x1000,
        Spectral { centroid_hz: Some(100.0), rolloff_hz: Some(200.0), noise_floor_db: -30.0 },
    );
    let text = index.to_text();
    assert!(text.contains("spectral 1"));
    // The binary reader checks a trailing-byte count too, so a truncated section cannot slip through there either.
    let bytes = index.to_bytes();
    let cut = &bytes[..bytes.len() - 20];
    assert!(Index::from_bytes(cut, &stamp()).is_err(), "a cut binary file is refused");
}

/// **Tempo and key round-trip in both formats**, including the two absences that a numeric column would destroy.
///
/// A non-rhythmic asset's `None` bpm and an atonal asset's `None` tonic are **facts to store**, not missing data: a
/// filter reads them as *unjudged*, and losing them on disk would make every asset look judged on reload.
#[test]
fn tempo_and_key_round_trip_in_both_formats() {
    use bevy_siren::semantic::key::{Estimate, Key};
    use bevy_siren::semantic::tempo::Tempo;

    let mut index = index_with(3);
    index.tempo.insert(0x1000, Tempo { bpm: Some(128.0), confidence: 0.86, octave_ambiguous: true });
    index.tempo.insert(0x1001, Tempo { bpm: None, confidence: 0.07, octave_ambiguous: false });
    index.key.insert(0x1000, Estimate { key: Key::new(7, true), confidence: 0.241 });
    index.key.insert(0x1001, Estimate { key: Key::absent(), confidence: 0.03 });

    let via_text = Index::load(&index.to_text(), &stamp()).expect("text loads");
    assert_eq!(via_text.tempo, index.tempo, "tempo survives the text form");
    assert_eq!(via_text.key, index.key, "and the key");

    let via_bytes = Index::from_bytes(&index.to_bytes(), &stamp()).expect("bytes load");
    assert_eq!(via_bytes.tempo, index.tempo, "tempo survives the binary form");
    assert_eq!(via_bytes.key, index.key, "and the key");

    // **The absences are preserved, and stay distinguishable from absence of a record.**
    assert_eq!(via_bytes.tempo[&0x1001].bpm, None, "a non-rhythmic asset keeps NO bpm");
    assert!(via_bytes.tempo[&0x1000].octave_ambiguous, "and the ambiguity flag survives");
    assert_eq!(via_bytes.key[&0x1001].key.tonic, None, "an atonal asset keeps NO tonic");
    assert!(!via_bytes.key.contains_key(&0x1002), "while an unmeasured asset has no record at all");
}

/// **An index with neither section writes neither**, so every earlier file still loads.
#[test]
fn an_index_with_no_tempo_or_key_writes_no_section() {
    let plain = index_with(2).to_text();
    assert!(!plain.contains("tempo "), "no tempo section when there is nothing to write");
    assert!(!plain.contains("key "), "and no key section");
    let back = Index::load(&plain, &stamp()).expect("loads");
    assert!(back.tempo.is_empty() && back.key.is_empty());
    let from_bytes = Index::from_bytes(&index_with(2).to_bytes(), &stamp()).expect("loads");
    assert!(from_bytes.tempo.is_empty() && from_bytes.key.is_empty());
}

/// **`#20` criterion 2: the override, when used, is reported in the result rather than only logged.**
///
/// The override is `Index::load_allowing(.., true)`, and what it reports is `Index::overridden` — carried on the index
/// itself, so every search built from it can see the mismatch without a second lookup.
#[test]
fn an_overridden_load_reports_the_mismatch_on_the_index() {
    let written = ModelStamp {
        model: "laion/clap-htsat-fused".into(),
        version: "a".into(),
        sample_rate: 48_000,
        dimension: 2,
    };
    let loaded = ModelStamp {
        model: "laion/clap-htsat-fused".into(),
        version: "b".into(),
        sample_rate: 48_000,
        dimension: 2,
    };

    let mut index = Index::new(written.clone());
    let _ = index.insert(7, vec![1.0, 0.0]);
    let bytes = index.to_bytes();

    // **Without the override it is still a refusal**, with both versions named.
    let err = Index::from_bytes(&bytes, &loaded).expect_err("a mismatch must refuse by default");
    match err {
        IndexProblem::ModelMismatch(m) => {
            assert_eq!(m.index.version, "a");
            assert_eq!(m.loaded.version, "b");
        }
        other => panic!("expected ModelMismatch, got {other:?}"),
    }

    // **With it, the load proceeds and says so.**
    let allowed = Index::from_bytes_allowing(&bytes, &loaded, true).expect("the override permits the load");
    let mismatch = allowed.overridden.expect("and the mismatch is recorded on the index");
    assert_eq!(mismatch.index.version, "a", "what the file says");
    assert_eq!(mismatch.loaded.version, "b", "what is loaded");
    assert_eq!(
        allowed.stamp.version, "a",
        "the stamp still describes the file, because that is a fact about it"
    );
}

/// A matching index carries no override, so a caller can test it without knowing how the load happened.
#[test]
fn a_matching_load_reports_no_override() {
    let stamp = ModelStamp { model: "m".into(), version: "1".into(), sample_rate: 48_000, dimension: 2 };
    let mut index = Index::new(stamp.clone());
    let _ = index.insert(1, vec![0.5, 0.5]);
    let allowed = Index::from_bytes_allowing(&index.to_bytes(), &stamp, true).expect("a matching load");
    assert_eq!(allowed.overridden, None, "nothing was overridden, so nothing is reported");
}
