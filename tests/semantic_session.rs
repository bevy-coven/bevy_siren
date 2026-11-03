//! A session: the budget decides first, the shape is checked, and a fixed-batch graph is honest about it.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::budget::{Budget, BudgetProblem, ModelCost};
use bevy_siren::semantic::session::{LoadProblem, Loaded, Session};

const GIB: u64 = 1024 * 1024 * 1024;

fn audio_like() -> Loaded {
    // What the real audio tower reported: a fixed batch axis, 512-d output.
    Loaded {
        inputs: vec!["input_features".into(), "is_longer".into()],
        outputs: vec!["audio_embed".into()],
        dimension: EMBEDDING_DIM,
        dynamic_batch: false,
    }
}

fn text_like() -> Loaded {
    // What the real text tower reported: `[-1, -1]`, so the batch axis is symbolic.
    Loaded {
        inputs: vec!["input_ids".into(), "attention_mask".into()],
        outputs: vec!["text_embed".into()],
        dimension: EMBEDDING_DIM,
        dynamic_batch: true,
    }
}

fn roomy() -> Budget {
    Budget { total_bytes: 24 * GIB, used_bytes: 0, ceiling_bytes: None }
}

fn cost() -> ModelCost {
    ModelCost { weights_bytes: 116_500_000, overhead_bytes: 512 * 1024 * 1024 }
}

#[test]
fn a_model_that_fits_is_admitted_and_keeps_its_cost() {
    let s = Session::admit(&roomy(), cost(), audio_like(), EMBEDDING_DIM).expect("admits");
    assert_eq!(s.cost(), cost(), "the cost it committed is readable afterwards");
    assert_eq!(s.loaded().dimension, EMBEDDING_DIM);
}

#[test]
fn a_budget_refusal_happens_before_any_device_work() {
    // The point of admitting through the budget: a refusal must not have touched a driver.
    let tight = Budget { total_bytes: GIB, used_bytes: GIB / 2, ceiling_bytes: None };
    let err = Session::admit(&tight, cost(), audio_like(), EMBEDDING_DIM).expect_err("refused");
    assert!(matches!(err, LoadProblem::Budget(BudgetProblem::DoesNotFit { .. })), "got {err:?}");
    assert!(err.to_string().contains("MB"), "the budget's own message survives: {err}");
}

#[test]
fn a_graph_of_the_wrong_width_is_refused_before_it_can_rank_anything() {
    // A 256-d graph against a 512-d index would compare a truncated vector to a full one.
    let mut wrong = audio_like();
    wrong.dimension = 256;
    let err = Session::admit(&roomy(), cost(), wrong, EMBEDDING_DIM).expect_err("wrong width");
    match err {
        LoadProblem::DimensionMismatch { graph, expected } => {
            assert_eq!(graph, 256);
            assert_eq!(expected, EMBEDDING_DIM);
        }
        other => panic!("expected a dimension mismatch, got {other:?}"),
    }
    assert!(err.to_string().contains("Re-index"), "{err}");
}

#[test]
fn a_fixed_batch_graph_admits_only_batch_one() {
    // Measured: CLAP's audio tower fails above batch 1 because patch_embed bakes the batch axis into a Reshape.
    // Reporting a larger capacity here would be the optimistic answer and the one that fails at run time.
    let s = Session::admit(&roomy(), cost(), audio_like(), EMBEDDING_DIM).expect("admits");
    assert_eq!(
        s.batch_sizes(&roomy(), 64 * 1024 * 1024),
        vec![1],
        "a fixed axis means exactly one batch size"
    );
}

#[test]
fn a_dynamic_batch_graph_admits_a_range() {
    // Measured: the text tower reports [-1, -1], so it takes a range.
    let s = Session::admit(&roomy(), cost(), text_like(), EMBEDDING_DIM).expect("admits");
    let sizes = s.batch_sizes(&roomy(), 1024 * 1024);
    assert!(sizes.len() > 1, "a symbolic axis takes more than one size: {sizes:?}");
    assert_eq!(sizes.first(), Some(&1), "and one is always among them");
}

#[test]
fn the_input_and_output_names_are_the_graphs_own() {
    // A caller binding `input_features` on the text graph gets a runtime error; keeping the names makes the test that
    // catches a mismatched pair possible.
    let a = Session::admit(&roomy(), cost(), audio_like(), EMBEDDING_DIM).expect("admits");
    assert_eq!(a.loaded().inputs, vec!["input_features", "is_longer"]);
    assert_eq!(a.loaded().outputs, vec!["audio_embed"]);

    let t = Session::admit(&roomy(), cost(), text_like(), EMBEDDING_DIM).expect("admits");
    assert_eq!(t.loaded().inputs, vec!["input_ids", "attention_mask"]);
}
