//! The evaluation arithmetic, against orders a person can check by eye.
//!
//! No model, no index, no device: `score` takes rankings a caller supplies, so what is tested here is the *metric* —
//! which is what decides whether a later ranking change counts as a regression.

#![cfg(feature = "semantic")]

use std::collections::BTreeMap;

use bevy_siren::semantic::eval::{Judged, Labelled, Ranking, agreement, at_floor, recall_at_k, score};

fn ranking(order: &[u64]) -> Ranking {
    Ranking { order: order.to_vec() }
}

#[test]
fn recall_counts_one_per_description_within_k() {
    let r = ranking(&[0xa0, 0xb0, 0xc0]);
    assert_eq!(recall_at_k(&r, 0xa0, 1), 1.0, "first within 1");
    assert_eq!(recall_at_k(&r, 0xb0, 1), 0.0, "second is not within 1");
    assert_eq!(recall_at_k(&r, 0xb0, 2), 1.0, "second within 2");
    assert_eq!(recall_at_k(&r, 0xd0, 3), 0.0, "absent is a miss however large k is");
}

#[test]
fn recall_uses_a_prefix_not_a_membership_test() {
    // The distinction a careless implementation loses: a result at rank 9 is not "found" by k = 3.
    let r = ranking(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 42]);
    assert_eq!(recall_at_k(&r, 42, 3), 0.0);
    assert_eq!(recall_at_k(&r, 42, 10), 1.0);
}

#[test]
fn agreement_is_the_share_of_affirmed_neighbours_above_the_anchor() {
    // Anchor at index 2; three affirmed neighbours, two above it and one below.
    let r = ranking(&[10, 11, 99, 12]);
    let judged = Judged { anchor: 99, fits: vec![10, 11, 12] };
    let a = agreement(&r, &judged);
    assert!((a - 2.0 / 3.0).abs() < 1e-6, "two of three above the anchor, got {a}");
}

#[test]
fn an_anchor_absent_from_its_own_ranking_scores_zero() {
    // Every judgement about it is unusable: nothing can be placed relative to a thing that is not there.
    let r = ranking(&[10, 11]);
    let judged = Judged { anchor: 99, fits: vec![10, 11] };
    assert_eq!(agreement(&r, &judged), 0.0);
}

#[test]
fn a_neighbour_absent_from_the_ranking_counts_as_a_miss() {
    // "Not placed" and "placed below" look the same to a listener, and the metric must not reward the first.
    let r = ranking(&[10, 99]);
    let judged = Judged { anchor: 99, fits: vec![10, 77] };
    let a = agreement(&r, &judged);
    assert!((a - 0.5).abs() < 1e-6, "one of two found above, got {a}");
}

#[test]
fn an_anchor_with_no_judgements_is_not_a_failure() {
    let r = ranking(&[99]);
    let judged = Judged { anchor: 99, fits: vec![] };
    assert_eq!(agreement(&r, &judged), 1.0, "nothing affirmed, nothing missed");
}

#[test]
fn a_perfect_set_says_so_in_one_sentence() {
    let labelled = vec![Labelled { description: "a low rumble".into(), expected: 0xa0 }];
    let mut rankings = BTreeMap::new();
    rankings.insert("a low rumble".to_string(), ranking(&[0xa0, 0xb0]));
    let report = score(&labelled, &rankings, &[], 3);
    assert!(report.perfect(), "{}", report.describe());
    assert_eq!(report.recall(), 1.0);
    assert!(report.describe().contains("recall@k 100%"), "{}", report.describe());
}

#[test]
fn a_miss_names_the_description_and_its_actual_rank() {
    let labelled = vec![
        Labelled { description: "a low rumble".into(), expected: 0xa0 },
        Labelled { description: "white noise hiss".into(), expected: 0xc0 },
    ];
    let mut rankings = BTreeMap::new();
    rankings.insert("a low rumble".to_string(), ranking(&[0xa0, 0xb0]));
    rankings.insert("white noise hiss".to_string(), ranking(&[0xb0, 0xa0, 0xc0]));
    let report = score(&labelled, &rankings, &[], 2);

    assert_eq!(report.recalled, 1);
    assert!(!report.perfect());
    let (name, at) = report.missed.first().expect("one miss");
    assert_eq!(name, "white noise hiss");
    assert_eq!(*at, Some(2), "it is at rank 2, outside k = 2");
    let s = report.describe();
    assert!(s.contains("white noise hiss"), "the failure is named: {s}");
    assert!(s.contains("rank 2"), "and its rank: {s}");
}

#[test]
fn a_description_with_no_ranking_is_a_miss_named_absent() {
    // The run did not cover it, which is different from "ranked badly" and must read differently.
    let labelled = vec![Labelled { description: "a cave".into(), expected: 0xa0 }];
    let report = score(&labelled, &BTreeMap::new(), &[], 3);
    assert_eq!(report.recalled, 0);
    assert_eq!(report.missed.first().map(|(_, at)| *at), Some(None));
    assert!(report.describe().contains("absent"), "{}", report.describe());
}

#[test]
fn the_disagreeing_anchors_are_named_with_their_figures() {
    let good = Judged { anchor: 99, fits: vec![10] };
    let bad = Judged { anchor: 77, fits: vec![20, 21] };
    let anchors = vec![(good, ranking(&[10, 99])), (bad, ranking(&[22, 77, 20, 21]))];
    let report = score(&[], &BTreeMap::new(), &anchors, 3);
    assert_eq!(report.anchors, 2);
    assert!(report.mean_agreement < 1.0, "one anchor disagreed");
    assert_eq!(report.disagreed.len(), 1, "only the bad one is named");
    assert_eq!(report.disagreed[0].0, 77);
    assert!(report.describe().contains("Disagreed"), "{}", report.describe());
}

#[test]
fn a_floor_that_hides_the_answer_turns_a_hit_into_a_miss() {
    // **The question V-01 exists to answer.** The right answer scores 0.126; at a 0.5 floor it is dropped before any
    // caller sees it, so recall falls from 1.0 to 0.0 for that description.
    let labelled = vec![
        Labelled { description: "a low rumble".into(), expected: 0xa0 },
        Labelled { description: "white noise hiss".into(), expected: 0xc0 },
    ];
    let mut scored = BTreeMap::new();
    scored.insert("a low rumble".to_string(), vec![(0xa0u64, 0.126f32), (0xb0, 0.08)]);
    scored.insert("white noise hiss".to_string(), vec![(0xc0u64, 0.429f32), (0xb0, 0.423)]);

    let none = at_floor(&labelled, &scored, 3, 0.0);
    assert_eq!(none.kept, 2, "with no floor both answers survive");
    assert_eq!(none.recall(), 1.0);

    let measured = at_floor(&labelled, &scored, 3, 0.126);
    assert_eq!(measured.kept, 2, "at the measured floor 0.126 both still survive");

    let intuitive = at_floor(&labelled, &scored, 3, 0.5);
    assert_eq!(intuitive.kept, 0, "at 0.5 — the number intuition reaches for — neither does");
    assert_eq!(intuitive.recall(), 0.0);
}

#[test]
fn a_floor_outcome_reports_the_floor_it_tested() {
    let labelled = vec![Labelled { description: "x".into(), expected: 1 }];
    let mut scored = BTreeMap::new();
    scored.insert("x".to_string(), vec![(1u64, 0.4f32)]);
    let o = at_floor(&labelled, &scored, 1, 0.25);
    assert_eq!(o.floor, 0.25, "the answer says which floor produced it");
    assert_eq!(o.kept, 1);
    assert_eq!(o.of, 1);
}

#[test]
fn an_empty_set_is_not_perfect() {
    // A run over nothing proves nothing, and `perfect()` must not read as a pass.
    let report = score(&[], &BTreeMap::new(), &[], 3);
    assert!(!report.perfect(), "no descriptions is not a perfect score");
    assert_eq!(report.recall(), 0.0);
}

#[test]
fn the_order_of_the_rankings_does_not_change_the_metric() {
    // Determinism of the metric itself: the same inputs give the same report, and inserting the rankings in a different
    // order gives the same report too.
    let labelled = vec![
        Labelled { description: "a".into(), expected: 1 },
        Labelled { description: "b".into(), expected: 2 },
    ];
    let mut one = BTreeMap::new();
    one.insert("a".to_string(), ranking(&[1, 3]));
    one.insert("b".to_string(), ranking(&[3, 2]));
    let mut two = BTreeMap::new();
    two.insert("b".to_string(), ranking(&[3, 2]));
    two.insert("a".to_string(), ranking(&[1, 3]));

    let a = score(&labelled, &one, &[], 2);
    let b = score(&labelled, &two, &[], 2);
    assert_eq!(a.describe(), b.describe());
    assert_eq!(a.recalled, b.recalled);
}

// ---------------------------------------------------------------------------------------------------------------
// The measurement this module exists for: what a floor does to the real numbers.
// ---------------------------------------------------------------------------------------------------------------

/// **What each candidate floor does to recall, on the cosines the real CLAP model produced.**
///
/// The four rows are `evidence/ask-e2e.txt` from iteration 68 — the real model, four clips, four descriptions — so this
/// is not a hypothetical set. The point is the shape of the answer: recall is **1.0 with no floor**, stays 1.0 at the
/// measured floor, and falls to **0.0** at the number intuition reaches for.
///
/// That is why `Floor::default()` is `0.0` and says so in its own doc comment: every floor inside the overlap deletes
/// right answers, and a long list a caller can weigh beats a short list that has silently dropped the answer.
/// (description, expected hash, the cosines CLAP actually produced, best first)
type Measured = (&'static str, u64, &'static [(u64, f32)]);

#[test]
fn a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model() {
    let measured: &[Measured] = &[
        ("a low rumble", 0xa0, &[(0xa0, 0.217), (0xb0, 0.081), (0xc0, 0.058)]),
        ("a steady musical tone", 0xb0, &[(0xb0, 0.150), (0xc0, -0.003), (0xa0, -0.027)]),
        ("white noise hiss", 0xc0, &[(0xc0, 0.429), (0xa0, 0.423), (0xb0, 0.026)]),
        ("rain falling", 0xa0, &[(0xa0, 0.126), (0xb0, 0.115), (0xc0, 0.077)]),
    ];
    let labelled: Vec<Labelled> =
        measured.iter().map(|(d, e, _)| Labelled { description: (*d).to_string(), expected: *e }).collect();
    let mut scored = BTreeMap::new();
    for (d, _, rows) in measured {
        scored.insert((*d).to_string(), rows.to_vec());
    }

    let no_floor = at_floor(&labelled, &scored, 1, 0.0);
    assert_eq!(no_floor.recall(), 1.0, "every description finds its answer with no floor");

    let measured_floor = at_floor(&labelled, &scored, 1, 0.126);
    assert_eq!(measured_floor.recall(), 1.0, "and still at the weakest correct match measured");

    let intuitive = at_floor(&labelled, &scored, 1, 0.5);
    assert_eq!(intuitive.kept, 0, "at 0.5, no description finds its answer at all");

    // And the documented constants bracket the trap: the correct band starts BELOW the incorrect band's ceiling, which
    // is decided when the crate is compiled rather than when this test runs.
    use bevy_siren::semantic::rank::{MEASURED_MATCH_FLOOR, MEASURED_MISS_CEILING};
    const {
        assert!(
            MEASURED_MATCH_FLOOR < MEASURED_MISS_CEILING,
            "the correct band must start below the incorrect band's ceiling: they overlap"
        )
    };
}
