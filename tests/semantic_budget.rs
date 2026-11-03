//! The GPU budget: what fits, what is refused, and the batch a leftover will take.
//!
//! Every number is in whole MiB so a person can check the arithmetic without a calculator.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::budget::{Budget, BudgetProblem, DEFAULT_OVERHEAD_BYTES, ModelCost};

const GIB: u64 = 1024 * 1024 * 1024;

fn cost(weights_mib: u64) -> ModelCost {
    ModelCost { weights_bytes: weights_mib * 1024 * 1024, overhead_bytes: 0 }
}

fn budget(total_gib: u64, used_mib: u64) -> Budget {
    Budget { total_bytes: total_gib * GIB, used_bytes: used_mib * 1024 * 1024, ceiling_bytes: None }
}

#[test]
fn a_model_that_fits_reports_ok() {
    // 24 GB card, 2 GB used, a 116 MB model: it fits with room to spare.
    let b = budget(24, 2048);
    assert_eq!(b.fits(cost(116)), Ok(()));
}

#[test]
fn a_model_that_does_not_fit_is_refused_with_both_numbers() {
    // 8 GB card, 7.9 GB already used: 116 MB does not fit in ~100 MB free.
    let b = budget(8, 8089);
    let err = b.fits(cost(116)).expect_err("must not fit");
    match err {
        BudgetProblem::DoesNotFit { need_bytes, available_bytes } => {
            assert_eq!(need_bytes, 116 * 1024 * 1024);
            assert!(available_bytes < need_bytes, "{available_bytes} < {need_bytes}");
        }
        other => panic!("expected DoesNotFit, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(msg.contains("MB"), "the message is in MB, which is what a person reads: {msg}");
    assert!(msg.contains("free"), "and says what is free: {msg}");
}

#[test]
fn a_ceiling_is_reported_as_the_projects_own_setting_not_the_cards() {
    // The card has room; this project's ceiling does not. The message must not blame the GPU.
    let b = Budget { total_bytes: 24 * GIB, used_bytes: 0, ceiling_bytes: Some(GIB) };
    let err = b.fits(cost(1200)).expect_err("over the ceiling");
    match err {
        BudgetProblem::OverCeiling { would_commit_bytes, ceiling_bytes } => {
            assert_eq!(ceiling_bytes, GIB);
            assert!(would_commit_bytes > ceiling_bytes);
        }
        other => panic!("expected OverCeiling, got {other:?}"),
    }
    assert!(err.to_string().contains("ceiling"), "the message names the ceiling: {err}");
}

#[test]
fn a_device_that_reports_more_used_than_total_is_caught() {
    let b = Budget { total_bytes: 8 * GIB, used_bytes: 9 * GIB, ceiling_bytes: None };
    let err = b.fits(cost(1)).expect_err("impossible numbers");
    assert!(matches!(err, BudgetProblem::Inconsistent { .. }), "got {err:?}");
}

#[test]
fn the_context_overhead_is_counted_beside_the_weights() {
    // The trap this exists for: a budget that ignores the context passes on the first model and fails on the second.
    // 24 GB card, 23.6 GB used. 116 MB of weights with no overhead would fit; with 512 MiB of context it must not.
    let b = budget(24, (23.6 * 1024.0) as u64);
    let weights_only = ModelCost { weights_bytes: 116_500_000, overhead_bytes: 0 };
    let with_context = ModelCost::measured_clap_audio();
    assert!(with_context.overhead_bytes == DEFAULT_OVERHEAD_BYTES);

    let free = b.available_bytes().expect("consistent");
    assert!(weights_only.total_bytes() <= free, "the weights alone fit in {} MB", free / 1024 / 1024);
    assert!(with_context.total_bytes() > free, "with the context they must not");
    assert!(b.fits(with_context).is_err(), "so the load is refused rather than failing inside the runtime");
}

#[test]
fn the_measured_model_carries_the_size_the_spike_reported() {
    let c = ModelCost::measured_clap_audio();
    assert_eq!(c.weights_bytes, 116_500_000, "the spike's onnx_mb, not a round number");
    assert_eq!(c.total_bytes(), 116_500_000 + DEFAULT_OVERHEAD_BYTES);
}

#[test]
fn a_batch_capacity_answers_at_least_one() {
    // Whatever the leftover, the answer is a batch, not zero: a refusal is `fits`'s decision.
    let b = Budget { total_bytes: GIB, used_bytes: 0, ceiling_bytes: None };
    assert_eq!(b.batch_capacity(ModelCost { weights_bytes: GIB, overhead_bytes: 0 }, 1024), 1);
    assert_eq!(
        b.batch_capacity(ModelCost { weights_bytes: 0, overhead_bytes: 0 }, 0),
        1,
        "no per-item cost means no limit"
    );
}

#[test]
fn a_batch_capacity_divides_the_leftover() {
    // 1 GiB free, a 256 MiB model, 64 MiB per item: 768 MiB left is 12 items.
    let b = Budget { total_bytes: GIB, used_bytes: 0, ceiling_bytes: None };
    let capacity = b.batch_capacity(cost(256), 64 * 1024 * 1024);
    assert_eq!(capacity, 12);
}

#[test]
fn a_ceiling_below_what_is_already_used_leaves_nothing_and_refuses() {
    // A project lowers its ceiling under current usage. Nothing fits, and it must say so rather than underflow.
    let b = Budget { total_bytes: 24 * GIB, used_bytes: 2 * GIB, ceiling_bytes: Some(GIB) };
    let available = b.available_bytes().expect("consistent");
    assert_eq!(available, 0, "a ceiling below usage leaves zero, not a wrapped number");
    assert!(b.fits(cost(1)).is_err());
}

#[test]
fn the_numbers_in_a_refusal_are_the_ones_the_budget_used() {
    // A message whose numbers differ from the decision is worse than no message.
    // 8 GB card with 7.95 GB used: ~50 MB free, so a 116 MB model is refused and both numbers matter.
    let b = budget(8, 8140);
    let c = cost(116);
    let err = b.fits(c).expect_err("refused");
    let free = b.device_free_bytes().expect("consistent");
    match err {
        BudgetProblem::DoesNotFit { need_bytes, available_bytes } => {
            assert_eq!(need_bytes, c.total_bytes(), "the need is the cost the caller passed");
            assert_eq!(available_bytes, free, "the availability is what the device reports");
        }
        other => panic!("{other:?}"),
    }
}
