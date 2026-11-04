//! The query contract, and the floor calibration the measurement produced.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::query::{Query, QueryProblem};
use bevy_siren::semantic::rank::{Floor, top_k};
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
