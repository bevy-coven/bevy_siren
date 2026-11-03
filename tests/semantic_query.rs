//! The query contract, and the floor calibration the measurement produced.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::query::{Query, QueryProblem};
use bevy_siren::semantic::rank::{Floor, MEASURED_MATCH_FLOOR, MEASURED_MISS_CEILING, top_k};
use bevy_siren::semantic::{EMBEDDING_DIM, MAX_TEXT_TOKENS, TEXT_EMBEDDING_DIM};

#[test]
fn a_description_is_trimmed_and_kept() {
    let q = Query::new("  tense, sparse, underwater ruins  ").expect("a description");
    assert_eq!(
        q.text(),
        "tense, sparse, underwater ruins",
        "trimmed, so a log reads back the way a person typed it"
    );
}

#[test]
fn an_empty_description_is_refused() {
    assert_eq!(Query::new("").unwrap_err(), QueryProblem::Empty);
    assert_eq!(Query::new("   \t\n ").unwrap_err(), QueryProblem::Empty);
    assert!(Query::new("").unwrap_err().to_string().contains("empty"));
}

#[test]
fn a_description_with_no_words_is_refused_rather_than_sent() {
    // A query the tower cannot attach meaning to returns an arbitrary ranking, which looks like a working feature.
    assert_eq!(Query::new("12345").unwrap_err(), QueryProblem::NoWords);
    assert_eq!(Query::new("--- ... !!!").unwrap_err(), QueryProblem::NoWords);
}

#[test]
fn a_description_longer_than_the_model_accepts_names_both_numbers() {
    let q = Query::new("a cave").expect("valid");
    assert!(q.check_tokens(4).is_ok(), "a short query is fine");
    let err = q.check_tokens(MAX_TEXT_TOKENS + 1).expect_err("too long");
    assert_eq!(err, QueryProblem::TooLong { need: MAX_TEXT_TOKENS + 1, limit: MAX_TEXT_TOKENS });
    let msg = err.to_string();
    assert!(msg.contains(&MAX_TEXT_TOKENS.to_string()), "{msg}");
    assert!(msg.contains("Shorten"), "the message says what to do: {msg}");
}

#[test]
fn the_token_limit_is_the_one_the_model_card_states() {
    assert_eq!(MAX_TEXT_TOKENS, 77);
    assert_eq!(TEXT_EMBEDDING_DIM, EMBEDDING_DIM, "text and audio must be the same width to be compared");
}

// ---------------------------------------------------------------------------------------------------------------
// The floor, calibrated against the measurement rather than against intuition.
// ---------------------------------------------------------------------------------------------------------------

#[test]
fn the_measured_bands_overlap_which_is_why_the_default_floor_keeps_everything() {
    // This test exists to stop someone "tidying" the default floor up to a number that feels right. The measurement
    // says a correct match can score lower than an incorrect one, so no single threshold separates them.
    // Two constants, so this is decided when the crate is compiled rather than when the test runs: if a later
    // measurement finds the bands separated, this stops the build and forces the default to be reconsidered.
    const {
        assert!(
            MEASURED_MATCH_FLOOR < MEASURED_MISS_CEILING,
            "the measured correct band starts below the incorrect band's ceiling: they overlap, so no floor separates them"
        )
    };
    assert_eq!(Floor::default().min_score, 0.0, "so the default keeps everything");
}

#[test]
fn an_intuitive_floor_would_have_rejected_every_correct_match() {
    // The number a person reaches for is 0.5. Measured: the best correct match scored 0.429.
    let intuitive = Floor { min_score: 0.5 };
    let correct_matches = [0.126f32, 0.133, 0.217, 0.218, 0.429];
    let kept = correct_matches.iter().filter(|s| **s >= intuitive.min_score).count();
    assert_eq!(kept, 0, "a 0.5 floor keeps none of the five correct matches");
}

#[test]
fn a_floor_at_the_measured_boundary_keeps_the_best_and_drops_the_worst_miss() {
    // Not a recommendation — a demonstration that the numbers behave. At 0.15 the weakest correct match is dropped and
    // the strongest miss is kept, which is exactly the overlap the default avoids committing to.
    let f = Floor { min_score: MEASURED_MISS_CEILING };
    assert!(0.126 < f.min_score, "the weakest correct match falls below it");
    assert!(0.150 >= f.min_score, "and the strongest miss sits at it");
}

#[test]
fn a_floor_still_works_on_a_real_ranking() {
    let mut index = Index::new(ModelStamp::measured_clap());
    let mut near = vec![0.0f32; EMBEDDING_DIM];
    near[0] = 1.0;
    let mut far = vec![0.0f32; EMBEDDING_DIM];
    far[1] = 1.0;
    index.insert(0x01, near.clone()).expect("insert");
    index.insert(0x02, far).expect("insert");

    let all = top_k(&index, &near, 5).expect("rank");
    assert_eq!(all.len(), 2);
    assert_eq!(Floor { min_score: 0.5 }.apply(all).len(), 1, "the orthogonal vector is dropped");
}
