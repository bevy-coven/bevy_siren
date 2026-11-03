//! Ranking: a cosine a person can check, a total order, and the four refusals.

#![cfg(feature = "semantic")]

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::rank::{Floor, RankProblem, cosine, top_k};

fn index() -> Index {
    Index::new(ModelStamp::measured_clap())
}

/// A unit vector along axis `i`, so its cosine with another axis vector is exactly 0 or 1.
fn axis(i: usize) -> Vec<f32> {
    let mut v = vec![0.0f32; EMBEDDING_DIM];
    v[i] = 1.0;
    v
}

#[test]
fn a_cosine_against_itself_is_one_and_against_an_orthogonal_axis_is_zero() {
    let a = axis(0);
    let b = axis(1);
    assert!((cosine(&a, &a).expect("nonzero") - 1.0).abs() < 1e-6);
    assert!(cosine(&a, &b).expect("nonzero").abs() < 1e-6);
}

#[test]
fn a_cosine_a_person_can_check_by_hand() {
    // (3,4) and (3,4): 1. (3,4) and (4,3): (12+12)/(5*5) = 0.96. (3,4) and (-3,-4): -1.
    let u = [3.0f32, 4.0];
    let v = [4.0f32, 3.0];
    let w = [-3.0f32, -4.0];
    assert!((cosine(&u, &u).expect("nonzero") - 1.0).abs() < 1e-6);
    assert!((cosine(&u, &v).expect("nonzero") - 0.96).abs() < 1e-6, "got {:?}", cosine(&u, &v));
    assert!((cosine(&u, &w).expect("nonzero") + 1.0).abs() < 1e-6);
}

#[test]
fn a_zero_vector_has_no_cosine_which_is_not_the_same_as_a_score_of_zero() {
    let zero = vec![0.0f32; EMBEDDING_DIM];
    let one = axis(0);
    assert_eq!(cosine(&zero, &one), None, "no direction, so no similarity");
    assert_eq!(cosine(&one, &zero), None);
    assert_eq!(cosine(&zero, &zero), None);
}

#[test]
fn a_non_finite_value_gives_no_cosine_rather_than_a_nan_score() {
    let mut v = axis(0);
    v[1] = f32::NAN;
    assert_eq!(cosine(&v, &axis(0)), None, "a NaN never becomes a score");
}

#[test]
fn a_query_of_the_wrong_length_is_refused_before_any_comparison() {
    let idx = index();
    let err = top_k(&idx, &[1.0, 0.0, 0.0], 3).expect_err("too short");
    assert_eq!(err, RankProblem::DimensionMismatch { query: 3, index: EMBEDDING_DIM });
}

#[test]
fn zero_k_is_refused_rather_than_answering_an_empty_list() {
    let idx = index();
    let err = top_k(&idx, &axis(0), 0).expect_err("k = 0");
    assert_eq!(err, RankProblem::ZeroK);
    assert!(err.to_string().contains("zero results"), "{err}");
}

#[test]
fn a_degenerate_vector_in_the_index_is_named_by_its_hash() {
    let mut idx = index();
    idx.insert(0xabc, axis(0)).expect("insert");
    idx.insert(0xdef, vec![0.0f32; EMBEDDING_DIM]).expect("a zero vector of the right length is storable");
    let err = top_k(&idx, &axis(0), 3).expect_err("a zero vector cannot be scored");
    assert_eq!(err, RankProblem::DegenerateVector { hash: Some(0xdef) });
    assert!(err.to_string().contains("0000"), "the hash is in the message: {err}");
}

#[test]
fn the_most_similar_asset_is_first() {
    let mut idx = index();
    idx.insert(0x01, axis(0)).expect("insert"); // identical to the query
    idx.insert(0x02, axis(1)).expect("insert"); // orthogonal
    idx.insert(0x03, axis(2)).expect("insert"); // orthogonal

    let results = top_k(&idx, &axis(0), 3).expect("rank");
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].hash, 0x01, "the identical vector is first");
    assert!((results[0].score - 1.0).abs() < 1e-6);
}

#[test]
fn ties_break_by_hash_so_two_runs_agree_field_for_field() {
    let mut idx = index();
    // Three equally-similar vectors at different hashes.
    for h in [0x30u64, 0x10, 0x20] {
        idx.insert(h, axis(0)).expect("insert");
    }
    let a = top_k(&idx, &axis(0), 3).expect("rank");
    let b = top_k(&idx, &axis(0), 3).expect("rank");
    assert_eq!(a, b, "the same query on the same index is the same answer");
    assert_eq!(a.iter().map(|m| m.hash).collect::<Vec<_>>(), vec![0x10, 0x20, 0x30], "ordered by hash");
}

#[test]
fn k_truncates_and_k_larger_than_the_index_returns_everything() {
    let mut idx = index();
    for i in 0..5 {
        idx.insert(i as u64, axis(i)).expect("insert");
    }
    assert_eq!(top_k(&idx, &axis(0), 2).expect("rank").len(), 2, "k truncates");
    assert_eq!(top_k(&idx, &axis(0), 99).expect("rank").len(), 5, "k beyond the index is not an error");
}

#[test]
fn an_empty_index_answers_with_nothing_not_an_error() {
    let idx = index();
    assert_eq!(top_k(&idx, &axis(0), 3).expect("an empty index is a valid query target"), vec![]);
}

#[test]
fn a_floor_drops_what_is_below_it() {
    let mut idx = index();
    idx.insert(0x01, axis(0)).expect("insert");
    idx.insert(0x02, axis(1)).expect("insert");
    let all = top_k(&idx, &axis(0), 5).expect("rank");
    assert_eq!(all.len(), 2);

    let floor = Floor { min_score: 0.5 };
    let kept = floor.apply(all);
    assert_eq!(kept.len(), 1, "the orthogonal vector is dropped");
    assert_eq!(kept[0].hash, 0x01);
}

#[test]
fn the_default_floor_keeps_everything_because_no_labelled_set_exists_yet() {
    // Design default, stated as such: a floor that hides the right answer is worse than a long list while `V-01`'s
    // labelled set does not exist.
    assert_eq!(Floor::default().min_score, 0.0);
    let mut idx = index();
    idx.insert(0x01, axis(0)).expect("insert");
    idx.insert(0x02, axis(1)).expect("insert");
    let all = top_k(&idx, &axis(0), 5).expect("rank");
    assert_eq!(Floor::default().apply(all).len(), 2);
}

/// **`#47`: the measured ranking is inside its stated budget**, so the constant cannot drift from the code.
///
/// This is a real timing test rather than a tautology, and it is sized to be stable on a shared machine: the budget is
/// **10 ms** at 10 000 assets against a measurement of **4.55 ms p99**, so a run has to be more than twice as slow as
/// this Mac before it fails. A budget that fired on a laptop would be a tripwire in the wrong place.
///
/// It ranks a **realistic corpus**: 10 000 vectors of 512 values, 20 queries, taking the p99 of the scan.
#[cfg(feature = "semantic")]
#[test]
fn the_ranking_is_inside_its_measured_budget() {
    use bevy_siren::semantic::index::{Index, ModelStamp};
    use bevy_siren::semantic::rank::{BUDGET_ASSETS, PER_QUERY_BUDGET_MS, top_k};

    fn vector(seed: u64) -> Vec<f32> {
        let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
        let mut out = Vec::with_capacity(512);
        for _ in 0..512 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            out.push((state >> 40) as f32 / 8_388_608.0 - 1.0);
        }
        out
    }

    let stamp =
        ModelStamp { model: "bench".into(), version: "1".into(), sample_rate: 48_000, dimension: 512 };
    let mut index = Index::new(stamp);
    for a in 0..BUDGET_ASSETS as u64 {
        let _ = index.insert(a, vector(a + 1));
    }

    let mut times = Vec::new();
    for q in 0..20u64 {
        let embedding = vector(1_000_000 + q);
        let start = std::time::Instant::now();
        let ranked = top_k(&index, &embedding, 3).expect("a ranking");
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        std::hint::black_box(&ranked);
    }
    times.sort_by(f64::total_cmp);
    let p99 = times[times.len() - 1];

    // **The budget is a released-build number, and the test says so rather than pretending otherwise.** Measured on this
    // Mac: **4.55 ms** in release, **86 ms** in debug — a 19x difference, because the cosine loop is arithmetic in a
    // 512-long vector and a debug build keeps the bounds checks and does not inline it. Asserting the release figure
    // against a debug build would fail on every run and teach a reader to ignore the test.
    //
    // So the assertion is release-only, and the debug run still measures and prints — a regression large enough to
    // matter still shows up in the number, and `cargo test --release` is what enforces it. The command is in the
    // module doc of `rank.rs` beside the constant.
    if cfg!(debug_assertions) {
        println!(
            "skipped: the budget is a released-build figure (measured 4.55 ms release, 86 ms debug); \
                  run `cargo test --release --features semantic` to enforce it. This run measured {p99:.2} ms."
        );
        return;
    }
    assert!(
        p99 <= PER_QUERY_BUDGET_MS,
        "the {BUDGET_ASSETS}-asset scan took {p99:.2} ms at p99, beyond the {PER_QUERY_BUDGET_MS} ms budget: \
         either the ranking regressed or the budget needs re-measuring"
    );
}
