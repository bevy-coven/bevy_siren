//! The whole pipeline, from a description to a ranked answer, with the model supplied as a slice.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::query::Query;
use bevy_siren::semantic::rank::{Floor, MEASURED_MATCH_FLOOR};
use bevy_siren::semantic::search::{SearchProblem, search};

fn stamp() -> ModelStamp {
    ModelStamp::measured_clap()
}

fn axis(i: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; EMBEDDING_DIM];
    v[i] = 1.0;
    v
}

/// A three-asset library: index 0 is what the query describes, 1 and 2 are other things.
fn library() -> Index {
    let mut idx = Index::new(stamp());
    idx.insert(0xa0, axis(0)).expect("insert");
    idx.insert(0xb0, axis(1)).expect("insert");
    idx.insert(0xc0, axis(2)).expect("insert");
    idx
}

#[test]
fn a_description_reaches_the_matching_asset() {
    let idx = library();
    let q = Query::new("tense, sparse, underwater ruins").expect("a description");
    let answer = search(&idx, &stamp(), &q, &axis(0), 3, Floor::default()).expect("search");
    assert_eq!(answer.matches.len(), 3);
    assert_eq!(answer.matches[0].hash, 0xa0, "the asset the embedding matches is first");
    assert_eq!(answer.dropped, 0);
}

#[test]
fn a_search_against_a_different_models_index_is_refused() {
    // The failure this prevents is a plausible ranking of noise: CLAP-512 and some-other-512 compare without complaint.
    let idx = library();
    let mut other = stamp();
    other.version = "a-different-revision".into();
    let q = Query::new("a cave").expect("a description");
    let err = search(&idx, &other, &q, &axis(0), 3, Floor::default()).expect_err("refused");
    assert_eq!(err, SearchProblem::StampMismatch);
    assert!(err.to_string().contains("different model"), "{err}");
}

#[test]
fn an_embedding_of_the_wrong_width_is_refused_with_both_numbers() {
    let idx = library();
    let q = Query::new("a cave").expect("a description");
    let err = search(&idx, &stamp(), &q, &[0.1; 10], 3, Floor::default()).expect_err("refused");
    assert_eq!(err, SearchProblem::WrongWidth { got: 10, expected: EMBEDDING_DIM });
}

#[test]
fn the_floor_drops_are_counted_rather_than_silent() {
    // A search that returns 1 of 3 must say so, or it is indistinguishable from a library with one asset.
    let idx = library();
    let q = Query::new("a cave").expect("a description");
    let answer = search(&idx, &stamp(), &q, &axis(0), 3, Floor { min_score: 0.5 }).expect("search");
    assert_eq!(answer.matches.len(), 1);
    assert_eq!(answer.dropped, 2);
    assert_eq!(answer.describe(), "1 results, 2 below the floor");
    assert!(!answer.filtered_out());
}

#[test]
fn everything_below_the_floor_says_so_rather_than_looking_empty() {
    let idx = library();
    let q = Query::new("a cave").expect("a description");
    let mut elsewhere = vec![0.0f32; EMBEDDING_DIM];
    elsewhere[5] = 1.0; // orthogonal to every stored vector
    let answer = search(&idx, &stamp(), &q, &elsewhere, 3, Floor { min_score: 0.5 }).expect("search");
    assert!(answer.matches.is_empty());
    assert!(answer.filtered_out());
    assert!(answer.describe().contains("below the floor"), "{}", answer.describe());
}

#[test]
fn an_empty_library_says_nothing_matched_rather_than_blaming_the_floor() {
    let idx = Index::new(stamp());
    let q = Query::new("a cave").expect("a description");
    let answer = search(&idx, &stamp(), &q, &axis(0), 3, Floor::default()).expect("search");
    assert!(answer.matches.is_empty());
    assert_eq!(answer.dropped, 0);
    assert_eq!(answer.describe(), "nothing in this library matched");
}

#[test]
fn the_same_description_and_index_always_answer_the_same_way() {
    // Determinism, end to end: the crate's rule, through the whole pipeline rather than only the ranking.
    let idx = library();
    let q = Query::new("a cave").expect("a description");
    let a = search(&idx, &stamp(), &q, &axis(0), 3, Floor::default()).expect("search");
    let b = search(&idx, &stamp(), &q, &axis(0), 3, Floor::default()).expect("search");
    assert_eq!(a, b, "field for field");
}

#[test]
fn a_measured_calibrated_floor_still_returns_the_match() {
    // The measured band says a correct match can score as low as 0.126, so a floor there must keep it.
    let idx = library();
    let q = Query::new("a cave").expect("a description");
    let answer =
        search(&idx, &stamp(), &q, &axis(0), 3, Floor { min_score: MEASURED_MATCH_FLOOR }).expect("search");
    assert_eq!(answer.matches.len(), 1, "the exact match survives its own floor");
    assert_eq!(answer.matches[0].score, 1.0);
}
