//! The ensemble arithmetic: a score a person can add up, and a breakdown that explains it.
//!
//! Every number here is checkable on paper. The module lives in `src/model.rs`, which `tests/all.rs` scans for engine
//! types, so the arithmetic is proved with no `App` — the rule the crate already keeps for its gains.

use bevy_siren::model::{Breakdown, Contribution, SIGNALS, Signal};

fn parts(semantic: f32, acoustic: f32, tempo: f32, key: f32) -> [Contribution; 4] {
    [
        Contribution::scored(Signal::Semantic, semantic, 1.0),
        Contribution::scored(Signal::Acoustic, acoustic, 0.5),
        Contribution::scored(Signal::Tempo, tempo, 0.35),
        Contribution::scored(Signal::Key, key, 0.35),
    ]
}

#[test]
fn the_score_is_the_sum_of_every_contribution() {
    // raw * weight: 0.4*1.0 + 0.2*0.5 + 0.3*0.35 + 0.1*0.35 = 0.4 + 0.1 + 0.105 + 0.035 = 0.64
    let b = Breakdown::new(parts(0.4, 0.2, 0.3, 0.1));
    assert!((b.score() - 0.64).abs() < 1e-6, "got {}", b.score());
}

#[test]
fn each_part_keeps_its_raw_value_its_weight_and_its_contribution() {
    // The three are different facts and a caller reading a result needs them separately.
    let b = Breakdown::new(parts(0.4, 0.2, 0.3, 0.1));
    let p = b.part(Signal::Acoustic);
    assert_eq!(p.raw, 0.2, "what the signal saw");
    assert_eq!(p.weight, 0.5, "how much the project cared");
    assert!((p.contribution - 0.1).abs() < 1e-6, "what entered the sum");
    assert!(!p.missing);
}

#[test]
fn a_missing_signal_is_not_a_zero_and_says_so() {
    // A tempo that was never estimated must not read as "tempo disagreed". Same for a key on an atonal asset.
    let b = Breakdown::new([
        Contribution::scored(Signal::Semantic, 0.4, 1.0),
        Contribution::scored(Signal::Acoustic, 0.2, 0.5),
        Contribution::absent(Signal::Tempo, 0.35),
        Contribution::absent(Signal::Key, 0.35),
    ]);
    assert!(b.part(Signal::Tempo).missing);
    assert_eq!(b.part(Signal::Tempo).contribution, 0.0, "contributes nothing");
    assert!(b.part(Signal::Key).raw.is_nan(), "and carries no value to misread");
    let e = b.explain();
    assert!(e.contains("no value for tempo, key"), "the sentence names them: {e}");
}

#[test]
fn a_non_finite_value_contributes_nothing_rather_than_poisoning_the_sum() {
    // `db_to_gain`'s rule, applied to a score: a NaN would make every comparison false and the ordering arbitrary.
    let b = Breakdown::new(parts(f32::NAN, 0.2, 0.3, 0.1));
    assert!(b.part(Signal::Semantic).missing, "a NaN is a missing signal");
    assert_eq!(b.part(Signal::Semantic).contribution, 0.0);
    assert!(b.score().is_finite(), "and the score is still a number: {}", b.score());
}

#[test]
fn a_negative_weight_contributes_nothing_rather_than_repelling() {
    let b = Breakdown::new([
        Contribution::scored(Signal::Semantic, 0.5, -1.0),
        Contribution::scored(Signal::Acoustic, 0.2, 0.5),
        Contribution::scored(Signal::Tempo, 0.3, 0.35),
        Contribution::scored(Signal::Key, 0.1, 0.35),
    ]);
    assert!(b.part(Signal::Semantic).missing, "a negative weight is not a preference, it is a sign error");
    assert!(b.score() > 0.0);
}

#[test]
fn the_shares_sum_to_one_and_say_what_the_result_was_made_of() {
    let b = Breakdown::new(parts(0.4, 0.2, 0.3, 0.1));
    let shares = b.shares().expect("a non-zero score has shares");
    let total: f32 = shares.iter().sum();
    assert!((total - 1.0).abs() < 1e-6, "the four shares sum to 1, got {total}");
    // 0.4 / 0.64 = 0.625
    assert!((shares[Signal::Semantic.index()] - 0.625).abs() < 1e-6, "got {}", shares[0]);
}

#[test]
fn a_zero_score_has_no_shares_rather_than_four_nans() {
    let b = Breakdown::new(parts(0.0, 0.0, 0.0, 0.0));
    assert_eq!(b.score(), 0.0);
    assert_eq!(b.shares(), None, "no share of nothing");
}

#[test]
fn a_penalty_is_kept_as_the_signal_that_counted_against() {
    // Loudness more than the tolerance away is a negative term, and it must survive into the breakdown rather than
    // being clamped at zero: a result "admitted despite" a penalty is a different result from an unpenalised one.
    let b = Breakdown::new([
        Contribution::scored(Signal::Semantic, 0.40, 1.0),
        Contribution::scored(Signal::Acoustic, -0.20, 0.5),
        Contribution::scored(Signal::Tempo, 0.30, 0.35),
        Contribution::scored(Signal::Key, 0.10, 0.35),
    ]);
    let (best, worst) = b.strongest().expect("something counted");
    assert_eq!(best.signal, Signal::Semantic, "the words carried it");
    let worst = worst.expect("a penalty is present");
    assert_eq!(worst.signal, Signal::Acoustic);
    assert!((worst.contribution + 0.1).abs() < 1e-6, "0.5 * -0.2 = -0.1, got {}", worst.contribution);

    let e = b.explain();
    assert!(e.contains("semantic carried it"), "{e}");
    assert!(e.contains("acoustic counted against it"), "{e}");
}

#[test]
fn the_explanation_names_the_raw_value_and_the_weight() {
    // "Why is this first" is answered by the numbers, not only by the name.
    let b = Breakdown::new(parts(0.4, 0.2, 0.3, 0.1));
    let e = b.explain();
    assert!(e.contains("+0.400 from 0.4 raw at weight 1"), "got {e}");
}

#[test]
fn a_breakdown_with_nothing_in_it_says_so_rather_than_naming_a_reason() {
    let b = Breakdown::new(parts(0.0, 0.0, 0.0, 0.0));
    assert_eq!(b.strongest(), None, "no signal contributed");
    assert!(b.explain().starts_with("no signal contributed"));
}

#[test]
fn ties_go_to_the_earlier_signal_so_two_runs_agree() {
    // Equal contributions, so the answer must be decided by the order rather than by luck.
    let b = Breakdown::new([
        Contribution::scored(Signal::Semantic, 0.5, 1.0),
        Contribution::scored(Signal::Acoustic, 1.0, 0.5),
        Contribution::scored(Signal::Tempo, 0.0, 0.35),
        Contribution::scored(Signal::Key, 0.0, 0.35),
    ]);
    let (best, _) = b.strongest().expect("something counted");
    assert_eq!(best.signal, Signal::Semantic, "0.5 and 0.5 tie, and semantic is earlier in SIGNALS");

    let again = Breakdown::new([
        Contribution::scored(Signal::Key, 0.0, 0.35),
        Contribution::scored(Signal::Tempo, 0.0, 0.35),
        Contribution::scored(Signal::Acoustic, 1.0, 0.5),
        Contribution::scored(Signal::Semantic, 0.5, 1.0),
    ]);
    assert_eq!(again.explain(), b.explain(), "the order the parts are given in cannot change the answer");
}

#[test]
fn the_signal_names_are_the_field_names_a_project_writes() {
    // A breakdown a person cross-references against their weights file must use the same words.
    assert_eq!(SIGNALS.map(Signal::name), ["semantic", "acoustic", "tempo", "key"]);
    assert_eq!(SIGNALS.map(Signal::index), [0, 1, 2, 3]);
}
