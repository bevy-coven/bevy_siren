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

/// **`#18` criterion 2: an out-of-memory refusal shrinks the batch, completes the list, and the shrink is reported.**
///
/// The embedder is injectable, so this is provable with no GPU: a fake that refuses any batch above three items behaves
/// exactly as a CUDA out-of-memory does from the caller's side.
#[test]
fn a_refusal_shrinks_the_batch_and_completes_the_list() {
    use bevy_siren::semantic::session::{Batched, embed_batched};

    let buffers: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32]).collect();
    let ceiling = 8;
    let mut attempted = Vec::new();
    let run: Result<Batched, _> = embed_batched(&buffers, ceiling, |batch| {
        attempted.push(batch.len());
        if batch.len() > 3 {
            return Err(String::from("CUDA_ERROR_OUT_OF_MEMORY"));
        }
        Ok(batch.to_vec())
    });
    let run = run.expect("the run completes by shrinking");

    // **The list is complete and in order**, which is the contract a batch that reorders would break.
    assert_eq!(run.vectors.len(), buffers.len());
    for (i, v) in run.vectors.iter().enumerate() {
        assert_eq!(v[0], i as f32, "the vectors are in the caller's order");
    }
    // **The shrink is reported**, and the sizes actually used are recorded — `#18`'s pitfall is a silent shrink.
    assert!(run.shrinks >= 1, "a refusal must be reported: {}", run.describe());
    assert!(run.largest_batch() <= 3, "no batch exceeded what the fake allows");
    assert!(run.describe().contains("shrink"), "the sentence names it: {}", run.describe());
    assert_eq!(run.batches.iter().sum::<usize>(), buffers.len());
    assert!(attempted.iter().any(|n| *n > 3), "it did try the ceiling first, and was refused");
}

/// **A refusal at size one is a failure, not a silent short answer.**
#[test]
fn a_refusal_at_one_item_is_reported_rather_than_shortened() {
    use bevy_siren::semantic::session::{BatchProblem, embed_batched};
    let buffers: Vec<Vec<f32>> = (0..4).map(|i| vec![i as f32]).collect();
    let run = embed_batched(&buffers, 8, |_batch| Err(String::from("the graph refused a single item")));
    match run {
        Err(BatchProblem::SingleItemRefused { why }) => assert!(why.contains("single item")),
        other => panic!("expected a failure, got {other:?}"),
    }
}

/// **A configured maximum of zero is refused**: it would embed nothing and report success.
#[test]
fn a_zero_batch_is_refused() {
    use bevy_siren::semantic::session::{BatchProblem, embed_batched};
    let run = embed_batched(&[vec![0.0]], 0, |batch| Ok(batch.to_vec()));
    assert_eq!(run, Err(BatchProblem::ZeroBatch));
}

/// **The same list embeds identically at batch 1 and at the ceiling** when the embedder is deterministic, which is the
/// half of `#18`'s first criterion that does not need a GPU. The GPU's own batched-versus-single tolerance is a
/// measurement `#18` still owes and is recorded as unmeasured in the issue.
#[test]
fn the_same_list_embeds_identically_at_any_batch_size() {
    use bevy_siren::semantic::session::embed_batched;
    let buffers: Vec<Vec<f32>> = (0..7).map(|i| vec![i as f32, -(i as f32)]).collect();
    let one = embed_batched(&buffers, 1, |batch| Ok(batch.to_vec())).expect("batch 1");
    let max = embed_batched(&buffers, 7, |batch| Ok(batch.to_vec())).expect("batch 7");
    assert_eq!(one.vectors, max.vectors, "deterministic means field for field, at any size");
    assert_eq!(one.batches.len(), 7, "batch 1 is seven batches");
    assert_eq!(max.batches.len(), 1, "the ceiling is one");
}

/// Recovery toward the ceiling after a shrink, so one early refusal does not cap the whole run.
#[test]
fn the_batch_recovers_toward_the_ceiling_after_a_shrink() {
    use bevy_siren::semantic::session::embed_batched;
    let buffers: Vec<Vec<f32>> = (0..16).map(|i| vec![i as f32]).collect();
    let mut first = true;
    let run = embed_batched(&buffers, 8, |batch| {
        if first {
            first = false;
            return Err(String::from("out of memory"));
        }
        if batch.len() > 4 {
            return Err(String::from("out of memory"));
        }
        Ok(batch.to_vec())
    })
    .expect("completes");
    assert!(run.shrinks >= 1);
    assert!(run.largest_batch() >= 4, "it recovered to what fits: {:?}", run.batches);
}
