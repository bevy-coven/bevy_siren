//! `Model`, against a real ONNX graph — a 16 KB stand-in with the CLAP text tower's shape contract.
//!
//! A permanent fixture rather than the 501 MB original: the point of this test is that the crate reads a graph's
//! **input names, output names, width and dynamic batch** from the file rather than assuming them, and a graph with
//! those properties proves it in 16 KB instead of half a gigabyte.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::model::Model;
use bevy_siren::semantic::session::LoadProblem;

fn fixture() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tiny-text.onnx")
}

#[test]
fn a_graphs_own_shape_is_read_from_the_file() {
    let m = Model::load(fixture()).expect("the fixture loads");
    let l = m.loaded();
    assert_eq!(l.inputs, vec!["input_ids", "attention_mask"], "the names are the graph's, not guessed");
    assert_eq!(l.outputs, vec!["text_embed"]);
    assert_eq!(l.dimension, EMBEDDING_DIM, "the last axis of the output shape");
    assert!(
        l.dynamic_batch,
        "the fixture declares [\"batch\", ...], so the batch axis is symbolic — the property that distinguishes the \
         text tower from the audio tower"
    );
}

#[test]
fn a_graph_runs_and_returns_unit_norm_values() {
    // The fixture normalises with ReduceL2 + Div, exactly as CLAP's projection head does, so this asserts the
    // property the ranker relies on: a cosine between two of these is a dot product.
    let mut m = Model::load(fixture()).expect("loads");
    let v = m.embed_text(&[101, 1996, 102], &[1, 1, 1]).expect("runs");
    assert_eq!(v.len(), EMBEDDING_DIM);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4, "the graph normalises its own output: norm is {norm}");
    assert!(v.iter().all(|x| x.is_finite()), "no NaN or infinity in a real run");
}

#[test]
fn ids_and_mask_of_different_lengths_are_refused_before_the_graph_runs() {
    let mut m = Model::load(fixture()).expect("loads");
    let err = m.embed_text(&[101, 102], &[1]).expect_err("mismatched inputs");
    assert!(err.to_string().contains("2 ids and 1 mask"), "{err}");
}

#[test]
fn a_missing_file_says_which_file() {
    let err = Model::load("/nonexistent/nope.onnx").expect_err("cannot load");
    match &err {
        LoadProblem::Unreadable { path, .. } => assert_eq!(path, "/nonexistent/nope.onnx"),
        other => panic!("expected Unreadable, got {other:?}"),
    }
    assert!(err.to_string().contains("/nonexistent/nope.onnx"), "{err}");
}

#[test]
fn a_file_that_is_not_a_graph_is_refused_rather_than_panicking() {
    let not_a_graph = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md");
    let err = Model::load(not_a_graph).expect_err("a README is not a model");
    assert!(matches!(err, LoadProblem::Unreadable { .. }), "got {err:?}");
}
