//! **The whole path in one call**: a description, a vocabulary, a model, an index, and a ranked answer.
//!
//! `Embedder::ask` exists because the four pieces were joinable in the wrong order in four ways and nothing said so. The
//! tests here are about the *ordering* of the refusals as much as about the answer, because the ordering is the part a
//! caller would otherwise get wrong: a description with no words in it must not reach the model, and an index from
//! another model must not be ranked.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::ask::{AskProblem, Embedder};
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::model::Model;
use bevy_siren::semantic::rank::Floor;
use bevy_siren::semantic::search::SearchProblem;
use bevy_siren::semantic::tokenizer::Vocabulary;

fn vocabulary() -> Vocabulary {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/clap-tokenizer.json");
    let text = std::fs::read_to_string(path).expect("the fixture is checked in");
    Vocabulary::from_json(&text).expect("CLAP's own tokenizer.json parses")
}

fn model() -> Model {
    // A 16 KB graph with the text tower's shape contract: `input_ids`/`attention_mask` as int64 [batch, tokens] with a
    // symbolic batch, `text_embed` as f32 [batch, 512], normalised by ReduceL2 + Div.
    Model::load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tiny-text.onnx"))
        .expect("the fixture loads")
}

fn stamp() -> ModelStamp {
    ModelStamp::measured_clap()
}

/// An index whose stored vectors are the fixture model's own output for three phrases, so a real query lands on one.
fn index_from(model: &mut Model) -> Index {
    let v = vocabulary();
    let mut index = Index::new(stamp());
    for (hash, text) in [(0xa0u64, "a low rumble"), (0xb0, "white noise hiss"), (0xc0, "a steady tone")] {
        let q = bevy_siren::semantic::query::Query::new(text).expect("a description");
        let ids: Vec<i64> = q.tokenize(&v).expect("tokenises").into_iter().map(i64::from).collect();
        let mask = vec![1i64; ids.len()];
        let embedding = model.embed_text(&ids, &mask).expect("embeds");
        index.insert(hash, embedding).expect("inserts");
    }
    index
}

#[test]
fn a_description_reaches_an_answer_in_one_call() {
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);

    let answer = embedder.ask(&index, &stamp(), "a low rumble", 3, Floor::default()).expect("answers");
    assert_eq!(answer.matches.len(), 3, "three indexed vectors are ranked");
    assert_eq!(answer.matches[0].hash, 0xa0, "the phrase the index was built from comes first");
}

#[test]
fn an_empty_description_is_refused_before_the_model_runs() {
    // The ordering claim: a query with nothing in it must not reach a GPU. Proven by the variant, because the model
    // below would otherwise succeed with it.
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);

    let err = embedder.ask(&index, &stamp(), "   ", 3, Floor::default()).expect_err("refused");
    assert_eq!(err, AskProblem::Query(bevy_siren::semantic::query::QueryProblem::Empty));
    assert!(err.to_string().contains("empty"), "{err}");
}

#[test]
fn a_description_with_no_words_is_refused_before_the_model_runs() {
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);
    let err = embedder.ask(&index, &stamp(), "12345 --- !!!", 3, Floor::default()).expect_err("refused");
    assert!(matches!(err, AskProblem::Query(_)), "got {err:?}");
}

#[test]
fn an_index_from_another_model_is_refused_before_any_embedding() {
    // The second ordering claim: a stamp mismatch must be caught before the encode, so a caller with the wrong file
    // learns it without paying for a model call.
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);

    let mut other = stamp();
    other.version = "a-different-revision".into();
    let err = embedder.ask(&index, &other, "a low rumble", 3, Floor::default()).expect_err("refused");
    assert_eq!(err, AskProblem::StampMismatch);
    assert!(err.to_string().contains("Re-index"), "the message says what to do: {err}");
}

#[test]
fn a_description_longer_than_the_model_accepts_is_refused_by_count() {
    use bevy_siren::semantic::MAX_TEXT_TOKENS;
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);
    let long = "a low rumble in the dark ".repeat(30);
    let err = embedder.ask(&index, &stamp(), &long, 3, Floor::default()).expect_err("refused");
    match &err {
        AskProblem::Query(bevy_siren::semantic::query::QueryProblem::TooLong { need, limit }) => {
            assert!(*need > MAX_TEXT_TOKENS);
            assert_eq!(*limit, MAX_TEXT_TOKENS);
        }
        other => panic!("expected TooLong, got {other:?}"),
    }
}

#[test]
fn the_answer_reports_what_the_floor_dropped() {
    // The fixture model averages its input ids, so every phrase gives the *same* unit vector and a high floor keeps
    // them all. What this asserts is the contract the caller relies on: `dropped` plus `matches` equals everything the
    // ranking found, and `describe()` says which.
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);

    let all = embedder.ask(&index, &stamp(), "a low rumble", 3, Floor::default()).expect("answers");
    let strict =
        embedder.ask(&index, &stamp(), "a low rumble", 3, Floor { min_score: 0.999 }).expect("answers");

    assert_eq!(all.matches.len() + all.dropped, 3, "the floor accounts for every ranked vector");
    assert_eq!(strict.matches.len() + strict.dropped, 3, "at any floor");
    assert!(
        strict.dropped >= all.dropped,
        "a stricter floor cannot keep more: {} dropped at 0.999, {} at 0.0",
        strict.dropped,
        all.dropped
    );
    assert!(!all.describe().is_empty(), "the Answer always has a sentence");
}

#[test]
fn the_same_description_answers_the_same_way_twice() {
    // Determinism end to end: the crate's rule, through the model as well as the ranking.
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);
    let a = embedder.ask(&index, &stamp(), "a low rumble", 3, Floor::default()).expect("answers");
    let b = embedder.ask(&index, &stamp(), "a low rumble", 3, Floor::default()).expect("answers");
    assert_eq!(a, b, "field for field");
}

#[test]
fn a_zero_k_is_still_a_refusal_at_the_top_of_the_path() {
    let v = vocabulary();
    let mut m = model();
    let index = index_from(&mut m);
    let mut embedder = Embedder::new(&v, &mut m);
    let err = embedder.ask(&index, &stamp(), "a low rumble", 0, Floor::default()).expect_err("refused");
    assert!(matches!(err, AskProblem::Search(SearchProblem::Rank(_))), "got {err:?}");
}

#[test]
fn an_embedding_is_the_width_the_index_holds() {
    // The property that makes the seam worth having: what `embed` returns can enter the index it will be searched in.
    let v = vocabulary();
    let mut m = model();
    let mut embedder = Embedder::new(&v, &mut m);
    let q = bevy_siren::semantic::query::Query::new("a low rumble").expect("a description");
    let embedding = embedder.embed(&q).expect("embeds");
    assert_eq!(embedding.len(), EMBEDDING_DIM);
}
