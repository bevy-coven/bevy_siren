//! The ensemble weights: their defaults, the refusals, and the RON round trip.
//!
//! Every default here is a **design default**, so what these tests pin is not "the number is right" but "the number is
//! documented, the report names what is wrong, and an unusual-but-possible configuration is allowed through".

#![cfg(feature = "semantic")]

use bevy_siren::semantic::weights::{WeightProblem, Weights};

#[test]
fn the_defaults_are_the_ones_the_docs_argue_for() {
    let w = Weights::default();
    assert_eq!(w.semantic, 1.0, "the unit the others are relative to");
    assert_eq!(w.acoustic, 0.5, "half the semantic weight");
    assert_eq!(w.tempo, 0.35, "below acoustic, because a tempo is absent for every effect");
    assert_eq!(w.key, 0.35, "equal to tempo, because nothing measures which matters more");
    assert_eq!(w.loudness_tolerance_db, 6.0, "a few dB is a mix decision");
    assert_eq!(w.drop_fraction, 0.0, "keep everything: a filter that hides the right answer is worse");
}

#[test]
fn the_default_configuration_has_nothing_wrong_with_it() {
    let w = Weights::default();
    assert_eq!(w.problems(), vec![]);
    assert!(w.is_usable());
}

#[test]
fn all_zero_weights_are_refused_with_a_message_that_says_what_to_do() {
    let w = Weights { semantic: 0.0, acoustic: 0.0, tempo: 0.0, key: 0.0, ..Weights::default() };
    let problems = w.problems();
    assert!(problems.contains(&WeightProblem::AllZero), "got {problems:?}");
    let msg = WeightProblem::AllZero.to_string();
    assert!(msg.contains("nothing would rank"), "{msg}");
    assert!(msg.contains("`semantic`"), "it names the fields to set: {msg}");
    assert!(!w.is_usable());
}

#[test]
fn a_negative_weight_names_the_field_and_the_sign_error() {
    let w = Weights { acoustic: -0.5, ..Weights::default() };
    match w.problems().first() {
        Some(WeightProblem::Negative { field, value }) => {
            assert_eq!(*field, "acoustic");
            assert_eq!(*value, -0.5);
        }
        other => panic!("expected Negative, got {other:?}"),
    }
    let msg = WeightProblem::Negative { field: "acoustic", value: -0.5 }.to_string();
    assert!(msg.contains("attract repel"), "the message explains why it is wrong: {msg}");
    assert!(msg.contains("Remove the minus sign"), "and what to do: {msg}");
}

#[test]
fn a_non_finite_weight_is_refused_rather_than_compared_false() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let w = Weights { semantic: bad, ..Weights::default() };
        assert!(
            w.problems().iter().any(|p| matches!(p, WeightProblem::NotFinite { field: "semantic", .. })),
            "{bad} must be refused, got {:?}",
            w.problems()
        );
    }
}

#[test]
fn a_tolerance_of_zero_or_less_is_refused() {
    // Zero would penalise every pair of sounds, which is the same as filtering the library to one asset.
    for bad in [0.0, -3.0] {
        let w = Weights { loudness_tolerance_db: bad, ..Weights::default() };
        assert!(
            w.problems().iter().any(|p| matches!(p, WeightProblem::BadTolerance { .. })),
            "{bad} must be refused"
        );
    }
}

#[test]
fn a_drop_fraction_outside_zero_to_one_is_refused() {
    for bad in [-0.1, 1.0, 1.5, f32::NAN] {
        let w = Weights { drop_fraction: bad, ..Weights::default() };
        assert!(
            w.problems().iter().any(|p| matches!(p, WeightProblem::BadDropFraction { .. })),
            "{bad} must be refused, got {:?}",
            w.problems()
        );
    }
    // And the boundary that IS allowed.
    let w = Weights { drop_fraction: 0.99, ..Weights::default() };
    assert!(w.is_usable(), "0.99 is a strange choice but a possible one");
}

#[test]
fn an_unusual_but_possible_configuration_is_allowed_through() {
    // **The refusal that must NOT fire.** The issue's own example: a stylised world whose loudness jumps are intended.
    // A tolerance of 40 dB and a semantic-only ranking are legitimate, and the report must not punish them.
    let stylised =
        Weights { loudness_tolerance_db: 40.0, acoustic: 0.0, tempo: 0.0, key: 0.0, ..Weights::default() };
    assert_eq!(stylised.problems(), vec![], "a semantic-only ranking is a choice, not an error");
    assert!(stylised.is_usable());

    let permissive = Weights { drop_fraction: 0.0, loudness_tolerance_db: 0.5, ..Weights::default() };
    assert!(permissive.is_usable(), "a tight tolerance is still a tolerance");
}

#[test]
fn every_problem_is_reported_not_just_the_first() {
    // A file with two mistakes should name both, so a person fixes them once.
    let w = Weights { semantic: -1.0, loudness_tolerance_db: 0.0, ..Weights::default() };
    let problems = w.problems();
    assert_eq!(problems.len(), 2, "both are named: {problems:?}");
    assert!(problems.iter().any(|p| matches!(p, WeightProblem::Negative { .. })));
    assert!(problems.iter().any(|p| matches!(p, WeightProblem::BadTolerance { .. })));
}

#[test]
fn the_shares_sum_to_one_and_say_what_a_result_was_made_of() {
    let w = Weights::default();
    let shares = w.shares().expect("the defaults are usable");
    let total = shares.semantic + shares.acoustic + shares.tempo + shares.key;
    assert!((total - 1.0).abs() < 1e-6, "the four shares sum to 1, got {total}");
    // 1.0 / (1.0 + 0.5 + 0.35 + 0.35) = 1/2.2
    assert!((shares.semantic - 1.0 / 2.2).abs() < 1e-6, "got {}", shares.semantic);
    assert!((shares.acoustic - 0.5 / 2.2).abs() < 1e-6);
}

#[test]
fn all_zero_weights_have_no_shares_rather_than_four_nans() {
    let w = Weights { semantic: 0.0, acoustic: 0.0, tempo: 0.0, key: 0.0, ..Weights::default() };
    assert_eq!(w.shares(), None, "no total, so no share");
}

#[cfg(feature = "serde")]
#[test]
fn the_weights_round_trip_through_ron() {
    let original = Weights {
        semantic: 1.5,
        acoustic: 0.25,
        tempo: 0.1,
        key: 0.4,
        loudness_tolerance_db: 3.0,
        noise_floor_tolerance_db: 15.0,
        tempo_tolerance_bpm: 2.5,
        compatibility: Default::default(),
        drop_fraction: 0.2,
        min_similarity: 0.15,
    };
    let text = original.to_ron().expect("serialises");
    let back = Weights::from_ron(&text).expect("parses");
    assert_eq!(back, original, "field for field");
}

#[cfg(feature = "serde")]
#[test]
fn a_file_written_before_a_field_existed_still_loads_with_that_fields_default() {
    // The promise in the type's doc: `#[serde(default)]` on every field means an old file gains defaults rather than
    // failing to parse. Stated because it is a promise, not an accident.
    let old = "(semantic: 2.0)";
    let w = Weights::from_ron(old).expect("an old file loads");
    assert_eq!(w.semantic, 2.0, "the field it set");
    assert_eq!(w.acoustic, Weights::default_acoustic(), "and the rest take their defaults");
    assert_eq!(w.drop_fraction, 0.0);
    assert_eq!(w.min_similarity, 0.0, "the floor a tool surface filters on takes its own default");
}

#[cfg(feature = "serde")]
#[test]
fn a_misspelled_field_is_refused_rather_than_silently_ignored() {
    // `deny_unknown_fields` — the crate's existing choice. A typo that is ignored is a setting that never took effect
    // and nobody knows why.
    let err = Weights::from_ron("(acustic: 0.5)").expect_err("a typo must not parse");
    assert!(err.contains("do not parse"), "{err}");
    assert!(err.contains("acustic") || err.contains("unknown"), "the message names it: {err}");
}

#[cfg(feature = "serde")]
#[test]
fn a_file_that_parses_but_means_nothing_is_caught_by_problems_not_by_the_parser() {
    // The two layers do different jobs: RON says "I could not read this", `problems()` says "I read it and it is
    // impossible". A caller needs both.
    let text = "(semantic: 0.0, acoustic: 0.0, tempo: 0.0, key: 0.0)";
    let w = Weights::from_ron(text).expect("it is valid RON");
    assert!(!w.is_usable(), "but it cannot rank anything");
    assert!(w.problems().contains(&WeightProblem::AllZero));
}

/// **The similarity floor is validated against the range a cosine can produce.**
///
/// `2.0` is not a stricter filter — it is an empty library that looks like a filter, which is the same silent-empty
/// failure the tool surface had when it could not set the floor at all.
#[test]
fn a_similarity_floor_beyond_a_cosine_is_refused() {
    use bevy_siren::semantic::weights::WeightProblem;

    // **The refusal that must not fire**: every value a cosine can actually reach, including both ends.
    for good in [-1.0, -0.5, 0.0, 0.5, 1.0] {
        let w = Weights { min_similarity: good, ..Weights::default() };
        assert!(
            !w.problems().iter().any(|p| matches!(p, WeightProblem::BadSimilarity { .. })),
            "{good} is inside a cosine's range: {:?}",
            w.problems()
        );
    }

    for bad in [1.5, -1.5, 2.0, f32::NAN, f32::INFINITY] {
        let w = Weights { min_similarity: bad, ..Weights::default() };
        let problems = w.problems();
        assert!(
            problems.iter().any(|p| matches!(p, WeightProblem::BadSimilarity { .. })),
            "{bad} must be refused: {problems:?}"
        );
        // And the sentence names the field it is about, rather than sending a person to edit `drop_fraction`.
        let said = problems
            .iter()
            .find(|p| matches!(p, WeightProblem::BadSimilarity { .. }))
            .expect("found")
            .to_string();
        assert!(said.contains("min_similarity"), "the message names its own field: {said}");
        assert!(!said.contains("drop_fraction"), "and not another one: {said}");
    }
}

/// **A compatibility multiplier's refusal names the multiplier, not `drop_fraction`.**
///
/// These three checks used to push `BadDropFraction`, so a project that set `clash_penalty: 1.5` was told
/// *"`drop_fraction` is 1.5"* — sent to edit a field it had not touched.
#[test]
fn a_bad_compatibility_multiplier_names_the_multiplier() {
    use bevy_siren::semantic::key::Compatibility;
    use bevy_siren::semantic::weights::WeightProblem;

    for (field, compatibility) in [
        ("clash_penalty", Compatibility { clash_penalty: 1.5, ..Compatibility::default() }),
        ("parallel_penalty", Compatibility { parallel_penalty: -0.1, ..Compatibility::default() }),
    ] {
        let w = Weights { compatibility, ..Weights::default() };
        let problems = w.problems();
        let named = problems.iter().find_map(|p| match p {
            WeightProblem::BadMultiplier { field: f, .. } => Some(*f),
            _ => None,
        });
        assert_eq!(named, Some(field), "the variant names the field: {problems:?}");
        let said = problems
            .iter()
            .find(|p| matches!(p, WeightProblem::BadMultiplier { .. }))
            .expect("found")
            .to_string();
        assert!(said.contains(field), "the message names it: {said}");
        assert!(!said.contains("drop_fraction"), "and never the wrong field: {said}");
    }
}
