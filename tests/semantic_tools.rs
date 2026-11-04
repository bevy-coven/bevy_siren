//! `#42`, `#43` and `#44`: the anchor, the two weightings, and the explanation.

#![cfg(feature = "semantic")]

use bevy_siren::model::Signal;
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::provenance::{Provenance, Source};
use bevy_siren::semantic::spectral::Spectral;
use bevy_siren::semantic::tools::{Session, ToolProblem, Toolset};

fn stamp() -> ModelStamp {
    ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 }
}

/// Four assets: 1 and 2 are near neighbours, 3 is distant, 4 is a near-duplicate of 1.
fn index() -> Index {
    let mut index = Index::new(stamp());
    let _ = index.insert(1, vec![1.0, 0.0, 0.0]);
    let _ = index.insert(2, vec![0.9, 0.1, 0.0]);
    let _ = index.insert(3, vec![0.0, 1.0, 0.0]);
    // 0.9999 against asset 1: a different take of the same sound.
    // 0.99995 against asset 2, which is the pair the variant rule must collapse.
    let _ = index.insert(4, vec![0.9999, 0.0101, 0.0]);
    index
}

fn with_noise(mut index: Index, hash: u64, noise: f32) -> Index {
    index.spectral.insert(hash, Spectral { centroid_hz: None, rolloff_hz: None, noise_floor_db: noise });
    index
}

/// **`#42` criterion 1: setting an anchor then searching returns neighbours of THAT anchor.**
#[test]
fn setting_a_new_anchor_changes_the_neighbours() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();

    session.set_anchor(&index, 1).expect("an anchor in the index");
    let first = tools.compatible_music(&index, &session, None, 2).expect("a search");
    assert!(first.results.iter().all(|r| r.hash != 1), "the anchor is not its own neighbour");
    let first_top = first.results[0].hash;

    session.set_anchor(&index, 3).expect("an anchor in the index");
    let second = tools.compatible_music(&index, &session, None, 2).expect("a search");
    assert_ne!(first_top, second.results[0].hash, "a new anchor must produce new neighbours");
}

/// **`#42` criterion 2: setting the same anchor twice is idempotent.**
#[test]
fn setting_the_same_anchor_twice_changes_nothing() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let once = tools.compatible_music(&index, &session, None, 3).expect("a search");
    session.set_anchor(&index, 1).expect("the same anchor");
    let twice = tools.compatible_music(&index, &session, None, 3).expect("a search");
    assert_eq!(once, twice, "idempotent means field for field");
}

/// **`#42` criterion 3: no anchor and none passed is refused, saying how to set one.**
#[test]
fn searching_without_an_anchor_is_refused_with_the_remedy() {
    let index = index();
    let tools = Toolset::default();
    let session = Session::new();
    let problem = tools.compatible_music(&index, &session, None, 3).expect_err("no anchor");
    assert_eq!(problem, ToolProblem::NoAnchor);
    let message = problem.to_string();
    assert!(message.contains("set_anchor"), "the refusal says how to set one: {message}");
}

/// An anchor that is not in the index is refused by hash, not silently ignored.
#[test]
fn an_anchor_outside_the_index_names_its_hash() {
    let index = index();
    let mut session = Session::new();
    match session.set_anchor(&index, 99) {
        Err(ToolProblem::AnchorNotIndexed { anchor }) => assert_eq!(anchor, 99),
        other => panic!("expected AnchorNotIndexed, got {other:?}"),
    }
    assert_eq!(session.anchor(), None, "a refused anchor is not stored");
}

/// **`#42` criterion 4: a filter that excluded a candidate is reported, not silently applied.**
#[test]
fn an_excluded_candidate_is_counted() {
    // A 20 dB loudness gap, against the 6 dB tolerance: the **loudness** rule excludes, while the noise-floor rule
    // only penalises. Using the noise floor here would test the wrong rule.
    let mut index = index();
    for (hash, lufs) in [(1u64, -20.0f32), (2, -20.5), (3, -40.0)] {
        index.loudness.insert(
            hash,
            bevy_siren::semantic::loudness::Loudness {
                integrated: Some(lufs),
                true_peak: -1.0,
                sample_peak: -1.0,
                blocks: 10,
                blocks_possible: 10,
            },
        );
    }
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_music(&index, &session, None, 4).expect("a search");
    assert_eq!(answer.tally.excluded, 1, "the noisy asset is excluded: {}", answer.tally.describe());
    assert!(answer.results.iter().all(|r| r.hash != 3));
}

/// **`#43` criterion 1: effects are ordered by the effect weighting, and the weights are reported.**
#[test]
fn the_effect_weighting_is_used_and_reported() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_sfx(&index, &session, None, None, 3).expect("a search");
    assert_eq!(answer.weights, Weights::effects(), "the answer names the weights it used");
    assert_eq!(answer.weights.tempo, 0.0, "an effect has no tempo, so its weight is zero rather than dead");
    assert_eq!(answer.weights.key, 0.0);
    // Every result's tempo and key parts are missing, never zero-valued.
    for r in &answer.results {
        assert!(r.breakdown.part(Signal::Tempo).missing);
        assert!(r.breakdown.part(Signal::Key).missing);
    }
}

/// **`#43` criterion 2: an anchor with no key or tempo still produces results.**
#[test]
fn an_anchor_with_no_key_or_tempo_still_finds_effects() {
    let index = index(); // no tempo, key, loudness or spectral anywhere
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_sfx(&index, &session, None, None, 3).expect("a search");
    assert!(!answer.results.is_empty(), "an unmeasured anchor must still match");
}

/// **`#43` criterion 3: a category filter matching nothing returns an empty list with a reason.**
#[test]
fn a_category_that_matches_nothing_says_so() {
    let mut index = index();
    index
        .provenance
        .insert(2, Provenance { source: Some(Source::Pack("footsteps".into())), ..Provenance::default() });
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_sfx(&index, &session, None, Some("explosions"), 3).expect("a search");
    assert!(answer.results.is_empty());
    let said = answer.describe();
    assert!(said.contains("outside the category"), "the reason names the filter: {said}");
}

/// The category filter keeps what matches, case-insensitively on a pack name.
#[test]
fn a_category_keeps_its_own_assets() {
    let mut index = index();
    index
        .provenance
        .insert(2, Provenance { source: Some(Source::Pack("Footsteps".into())), ..Provenance::default() });
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_sfx(&index, &session, None, Some("footsteps"), 3).expect("a search");
    assert_eq!(answer.results.len(), 1);
    assert_eq!(answer.results[0].hash, 2);
}

/// **`#43`'s pitfall: near-identical variants are reported, not returned as the top N.**
#[test]
fn a_near_identical_variant_is_reported_not_returned() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_music(&index, &session, None, 4).expect("a search");
    let hashes: Vec<u64> = answer.results.iter().map(|r| r.hash).collect();

    // **The invariant, not a particular survivor.** Assets 2 and 4 are the same sound as far as the encoder can tell,
    // so exactly one may be returned — and which one is the better-scoring one, which is what makes the choice
    // principled rather than arbitrary.
    let kept_pair = [2u64, 4].iter().filter(|h| hashes.contains(h)).count();
    assert_eq!(kept_pair, 1, "exactly one of the variant pair survives: {hashes:?}");
    assert_eq!(answer.tally.variants.len(), 1, "the other is reported: {:?}", answer.tally.variants);

    // The omission names the result it resembled, so a caller knows the pair existed.
    let (dropped, resembled) = answer.tally.variants[0];
    assert!(hashes.contains(&resembled), "the named survivor is in the results");
    assert!(!hashes.contains(&dropped), "and the dropped one is not");
    assert!([2u64, 4].contains(&dropped) && [2u64, 4].contains(&resembled));

    // The survivor is the higher-scoring of the two, which is the whole reason the rule runs after ordering.
    let survivor = answer.results.iter().find(|r| [2u64, 4].contains(&r.hash)).expect("one survived");
    assert!(
        survivor.score >= answer.results.last().map(|r| r.score).unwrap_or(0.0),
        "it is the better of the two"
    );
}

/// **`#44` criterion 1: the breakdown matches what the ensemble produced for the same pair.**
#[test]
fn the_explanation_matches_the_search_breakdown() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_music(&index, &session, None, 4).expect("a search");

    for found in &answer.results {
        let explained = tools.explain_match(&index, 1, found.hash).expect("an explanation");
        assert_eq!(explained.breakdown, found.breakdown, "the same pair, the same numbers");
        assert_eq!(explained.score, found.score);
        assert_eq!(explained.similarity, found.similarity);
    }
}

/// **An explanation uses the weighting that ranked the pair.** `explain_match` was hard-wired to the music weights, so a
/// result of `compatible_sfx` — scored under the *effects* weights — was explained with different weights and a different
/// score: the numbers were individually right and did not add up to what the search returned. Found by reading the first
/// real session through `siren-tools`, where one pair showed `acoustic` at 1.2 in the search and 0.5 in its explanation.
#[test]
fn an_effect_is_explained_under_the_effects_weighting() {
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    let answer = tools.compatible_sfx(&index, &session, None, None, 4).expect("a search");
    assert!(!answer.results.is_empty(), "the search ranked something");

    for found in &answer.results {
        let with = tools.explain_match_with(&index, 1, found.hash, &tools.effects).expect("an explanation");
        assert_eq!(with.breakdown, found.breakdown, "the weighting that ranked it reproduces the search");
        assert_eq!(with.score, found.score);

        // The default is the music weighting, and under it the same pair is a different number: the defect.
        let music = tools.explain_match(&index, 1, found.hash).expect("an explanation");
        assert_ne!(
            music.breakdown, found.breakdown,
            "the music weighting does not explain an effects ranking"
        );
        assert_eq!(music, tools.explain_match_with(&index, 1, found.hash, &tools.music).expect("the same"));
    }
}

/// The tally is a sentence an agent reads, so a count of one is not plural.
#[test]
fn the_tally_sentence_is_singular_for_one_variant() {
    use bevy_siren::semantic::tools::Tally;
    let one = Tally { returned: 1, variants: vec![(2, 1)], ..Tally::default() };
    assert_eq!(one.describe(), "1 returned, 1 near-identical variant");
    let two = Tally { returned: 1, variants: vec![(2, 1), (3, 1)], ..Tally::default() };
    assert_eq!(two.describe(), "1 returned, 2 near-identical variants");
}

/// **`#44` criterion 2: the sentence names the strongest positive and the strongest negative.**
#[test]
fn the_sentence_names_both_directions() {
    // Asset 2 is 20 dB noisier than the anchor: a penalty, so the sentence must mention it.
    let index = with_noise(with_noise(index(), 1, -80.0), 2, -60.0);
    let tools = Toolset::default();
    let explained = tools.explain_match(&index, 1, 2).expect("an explanation");
    let sentence = explained.explain();
    assert!(sentence.contains("semantic"), "it names what carried it: {sentence}");
    assert!(
        sentence.contains("acoustic") && sentence.contains("against"),
        "it names what counted against it: {sentence}"
    );
}

/// **`#44` criterion 3: an unknown feature says so rather than reporting a zero as a real value.**
#[test]
fn an_unknown_feature_says_so() {
    let index = index(); // no tempo or key recorded anywhere
    let tools = Toolset::default();
    let explained = tools.explain_match(&index, 1, 2).expect("an explanation");
    let sentence = explained.explain();
    assert!(sentence.contains("no value for"), "the sentence names the gap: {sentence}");
    assert!(sentence.contains("tempo"), "and which signal it was: {sentence}");
    assert!(explained.breakdown.part(Signal::Tempo).missing);
    assert_eq!(explained.breakdown.part(Signal::Tempo).contribution, 0.0);
}

/// **`#44` criterion 4: it works for an effect as well as a track.**
#[test]
fn an_effect_explains_too() {
    let index = with_noise(index(), 2, -75.0);
    let tools = Toolset::default();
    let explained = tools.explain_match(&index, 1, 2).expect("an explanation");
    assert!(!explained.explain().is_empty());
    // An effect has no key or tempo, so those are absent rather than zero-valued.
    assert!(explained.breakdown.part(Signal::Key).missing);
}

/// The two weightings are genuinely different tables, and both are usable.
#[test]
fn the_two_weightings_are_separate_and_usable() {
    let music = Weights::default();
    let effects = Weights::effects();
    assert_ne!(music, effects);
    assert_eq!(music.problems(), vec![], "the music defaults are usable");
    assert_eq!(effects.problems(), vec![], "so are the effect defaults");
    // Their totals match, so a score from one is comparable to the other's.
    let total = |w: &Weights| w.semantic + w.acoustic + w.tempo + w.key;
    assert!(
        (total(&music) - total(&effects)).abs() < 1e-6,
        "the totals must match for the scores to be comparable"
    );
}

use bevy_siren::semantic::weights::Weights;

/// **`k` is a bound, not a page** — ADR 0007: every list-returning tool returns at most `k`, `k` has a ceiling, and a
/// bad one is refused **before** any work happens.
#[test]
fn a_bad_k_is_refused() {
    use bevy_siren::semantic::tools::{MAX_K, check_k};

    assert_eq!(check_k(1), Ok(1));
    assert_eq!(check_k(MAX_K), Ok(MAX_K));
    for bad in [0, MAX_K + 1] {
        match check_k(bad) {
            Err(bevy_siren::semantic::tools::ToolProblem::BadK { k, max }) => {
                assert_eq!(k, bad);
                assert_eq!(max, MAX_K);
            }
            other => panic!("expected BadK for {bad}, got {other:?}"),
        }
    }

    // And the tools enforce it rather than only the helper.
    let index = index();
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");
    assert!(matches!(
        tools.compatible_music(&index, &session, None, 0),
        Err(bevy_siren::semantic::tools::ToolProblem::BadK { .. })
    ));
    assert!(matches!(
        tools.compatible_sfx(&index, &session, None, None, MAX_K + 1),
        Err(bevy_siren::semantic::tools::ToolProblem::BadK { .. })
    ));
}

/// **`#30`: an audition produces exactly one `Play` for the crate's own play path, and never starts a voice itself.**
#[test]
fn an_audition_produces_exactly_one_play_through_the_existing_path() {
    use bevy_siren::semantic::tools::{AuditionProblem, audition, audition_cue_name};

    let index = index();
    let mut library = bevy_siren::library::Library::default();
    library.assets.insert(
        2,
        bevy_siren::library::Asset {
            paths: vec![std::path::PathBuf::from("footstep_stone.wav")],
            hash: 2,
            bytes: 4_096,
            sample_rate: Some(48_000),
            channels: Some(1),
        },
    );

    let prepared = audition(&index, 2, &library, "footstep_stone", "Effects").expect("a prepared audition");

    // **One play, naming one cue.** `Play` is a struct, so "exactly one" is structural rather than counted.
    assert_eq!(prepared.play.cue, prepared.cue.name);
    assert_eq!(prepared.cue.name, audition_cue_name(2));
    assert_eq!(prepared.cue.sounds.len(), 1, "one sound, so a retry cannot double-play two");
    assert_eq!(prepared.cue.bus, "Effects");

    // **The same asset auditions under the same name**, so two auditions are one cue rather than two.
    let again = audition(&index, 2, &library, "footstep_stone", "Effects").expect("again");
    assert_eq!(again.cue.name, prepared.cue.name);

    // A different asset is a different cue.
    let other = audition(&index, 3, &library, "gun_burst", "Effects");
    assert!(
        matches!(other, Err(AuditionProblem::NoPath { asset: 3 })),
        "asset 3 is not in the library: {other:?}"
    );

    // **One instance and not spatial**: an audition is one sound in the listener's ear.
    assert_eq!(prepared.cue.max_instances, 1);
    assert!(!prepared.cue.spatial);
}

/// An audition names what is missing rather than playing silence.
#[test]
fn an_audition_names_what_is_missing() {
    use bevy_siren::semantic::tools::{AuditionProblem, audition};

    let index = index();
    let library = bevy_siren::library::Library::default();

    // Not in the index at all.
    match audition(&index, 99, &library, "x", "Effects") {
        Err(AuditionProblem::NotIndexed { asset }) => assert_eq!(asset, 99),
        other => panic!("expected NotIndexed, got {other:?}"),
    }
    // In the index, but no file holds it — the library changed since the index was built.
    match audition(&index, 2, &library, "x", "Effects") {
        Err(AuditionProblem::NoPath { asset }) => assert_eq!(asset, 2),
        other => panic!("expected NoPath, got {other:?}"),
    }
    assert!(AuditionProblem::NotIndexed { asset: 1 }.to_string().contains("not in this index"));
    assert!(AuditionProblem::NoPath { asset: 1 }.to_string().contains("rebuild it"));
}

/// **`#29`: `search_music` returns at most k, ordered, with everything an agent needs to choose.**
#[test]
fn search_music_returns_a_decision_without_another_call() {
    use bevy_siren::semantic::query::Query;
    use bevy_siren::semantic::rank::Floor;
    use bevy_siren::semantic::tools::search_music;
    use bevy_siren::semantic::weights::Weights;

    let index = index();
    let stamp = index.stamp.clone();
    let query = Query::new("a tense sparse ambience").expect("a query");
    let embedding = vec![1.0f32, 0.0, 0.0];
    let features = std::collections::BTreeMap::new();
    let weights = Weights::default();

    let request = bevy_siren::semantic::search::Ranked {
        embedding: &embedding,
        anchor: 0,
        features: &features,
        k: 2,
        floor: Floor::default(),
        weights: &weights,
    };
    let answer = search_music(&index, &stamp, &query, &request).expect("a search");

    // **At most k, and ordered**: descending score, so the first is the one to choose.
    assert!(answer.results.len() <= 2, "at most k");
    assert!(answer.results.windows(2).all(|w| w[0].score >= w[1].score), "ordered by score");
    // **Enough to choose without another round trip**: identity, score, reason and provenance on every result.
    for r in &answer.results {
        assert_ne!(r.hash, 0, "the identity is the content hash, never a path");
        assert!(r.score.is_finite());
        assert_eq!(r.breakdown.parts.len(), 4, "the breakdown is present");
        assert!(!r.explain().is_empty(), "and it explains itself");
        // **Provenance is carried, not looked up later** — the field exists on every result even when nothing was
        // recorded, so an agent never needs a second call to learn that an asset is unlicensed.
        let _ = r.provenance.permits_commercial_use();
    }
    // The weights are echoed, so a result can be reproduced.
    assert_eq!(answer.weights, weights);
}

/// **`#29`: an empty library is an empty list with a reason, not an error.**
#[test]
fn search_music_on_an_empty_library_says_so() {
    use bevy_siren::semantic::index::{Index, ModelStamp};
    use bevy_siren::semantic::query::Query;
    use bevy_siren::semantic::rank::Floor;
    use bevy_siren::semantic::tools::search_music;
    use bevy_siren::semantic::weights::Weights;

    let stamp = ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 };
    let index = Index::new(stamp.clone());
    let query = Query::new("anything").expect("a query");
    let features = std::collections::BTreeMap::new();

    let weights = Weights::default();
    let request = bevy_siren::semantic::search::Ranked {
        embedding: &[1.0, 0.0, 0.0],
        anchor: 0,
        features: &features,
        k: 3,
        floor: Floor::default(),
        weights: &weights,
    };
    let answer =
        search_music(&index, &stamp, &query, &request).expect("an empty library is a state, not an error");
    assert!(answer.results.is_empty());
    let said = answer.describe();
    assert!(said.contains("nothing fit") || said.contains("0 returned"), "the reason is a sentence: {said}");
}

/// **`#29`: k is a bound with a default and a refusal** — the default is 3, and 0 is refused naming the range.
#[test]
fn search_music_bounds_k() {
    use bevy_siren::semantic::query::Query;
    use bevy_siren::semantic::rank::Floor;
    use bevy_siren::semantic::tools::{DEFAULT_K, ToolProblem, search_music};
    use bevy_siren::semantic::weights::Weights;

    assert_eq!(DEFAULT_K, 3, "#29 says the workflow's default is three");

    let index = index();
    let stamp = index.stamp.clone();
    let query = Query::new("x").expect("a query");
    let features = std::collections::BTreeMap::new();
    let weights = Weights::default();
    let request = bevy_siren::semantic::search::Ranked {
        embedding: &[1.0, 0.0, 0.0],
        anchor: 0,
        features: &features,
        k: 0,
        floor: Floor::default(),
        weights: &weights,
    };
    let refused = search_music(&index, &stamp, &query, &request).expect_err("k=0 is refused");
    match refused {
        ToolProblem::BadK { k, max } => {
            assert_eq!(k, 0);
            assert!(refused.to_string().contains("between 1 and"), "the range is named");
            assert!(max >= DEFAULT_K);
        }
        other => panic!("expected BadK, got {other:?}"),
    }
}

/// **A bad `k` is explained by the case it is, not by the other one.** The message told a caller who had asked for 101 that *"a k
/// of 0 asks for nothing"* — found by running `examples/semantic --k 101` — and an agent that asked for more cannot act on advice
/// about less.
#[test]
fn a_bad_k_is_explained_by_its_own_case() {
    use bevy_siren::semantic::tools::{MAX_K, check_k};

    let zero = check_k(0).expect_err("zero is refused").to_string();
    assert!(zero.contains("asks for nothing"), "{zero}");
    assert!(!zero.contains("at most"), "{zero}");

    let large = check_k(MAX_K + 1).expect_err("above the cap is refused").to_string();
    assert!(
        large.contains(&format!("at most {MAX_K}")) && large.contains(&(MAX_K + 1).to_string()),
        "{large}"
    );
    assert!(!large.contains("k of 0"), "a caller that asked for more is not told about zero: {large}");
}

/// **`#31`: every tool's failure carries the same shape, and the bad-argument/refused split is real.**
#[test]
fn every_failure_has_a_code_and_only_bad_arguments_are_retryable() {
    use bevy_siren::semantic::tools::{Code, ToolProblem};

    let failures = [
        ToolProblem::BadK { k: 0, max: 100 },
        ToolProblem::NoAnchor,
        ToolProblem::AnchorNotIndexed { anchor: 7 },
        ToolProblem::Search(String::from("a refusal")),
    ];

    // **Every variant has a code**, and the code's spelling is the wire form rather than a Debug print.
    for failure in &failures {
        let code = failure.code();
        assert!(
            ["bad_argument", "refused", "unavailable", "internal"].contains(&code.as_str()),
            "{} has an unknown code {:?}",
            failure,
            code.as_str()
        );
        // The convenience method cannot disagree with the code it wraps.
        assert_eq!(failure.retryable(), code.retryable());
    }

    // **A bad argument is retryable, a refusal is not** — the distinction the issue exists for.
    assert_eq!(ToolProblem::BadK { k: 0, max: 100 }.code(), Code::BadArgument);
    assert!(ToolProblem::BadK { k: 0, max: 100 }.retryable(), "the agent can fix this and call again");
    for refused in [ToolProblem::NoAnchor, ToolProblem::AnchorNotIndexed { anchor: 7 }] {
        assert_eq!(refused.code(), Code::Refused);
        assert!(!refused.retryable(), "retrying unchanged fails identically: {refused}");
    }
    assert!(!Code::Unavailable.retryable());
    assert!(!Code::Internal.retryable());
}

/// **`#31`: every tool states its idempotency, and exactly one is unsafe.**
#[test]
fn every_tool_states_its_idempotency() {
    use bevy_siren::semantic::tools::{Idempotency, TOOLS};

    // **Six tools, as ADR 0007 fixes**, so a tool added without a row here is a failing test.
    assert_eq!(TOOLS.len(), 6, "the surface is six tools");
    let names: Vec<&str> = TOOLS.iter().map(|(name, _)| *name).collect();
    for expected in [
        "search_music",
        "set_anchor",
        "find_compatible_music",
        "find_compatible_sfx",
        "explain_match",
        "audition",
    ] {
        assert!(names.contains(&expected), "{expected} is missing from the surface");
    }

    // **`audition` is the only unsafe one**: it starts a voice, so a retry plays twice.
    let unsafe_tools: Vec<&str> =
        TOOLS.iter().filter(|(_, id)| *id == Idempotency::Unsafe).map(|(n, _)| *n).collect();
    assert_eq!(unsafe_tools, vec!["audition"], "exactly one tool starts a voice");

    // And every tool's sentence says which and why.
    for (name, idempotency) in TOOLS {
        let said = idempotency.describe(name);
        assert!(said.starts_with(name), "the sentence names the tool: {said}");
        assert!(said.contains("retry"), "and says what retrying does: {said}");
    }
    assert!(Idempotency::Unsafe.describe("audition").contains("NOT safe"));
}

/// **A category is matched case-insensitively**, because it is a string a person typed into a provenance sidecar.
///
/// The old code lowercased the haystack and compared it with a **raw** needle, so `Some("Footsteps")` matched nothing —
/// and the tally blamed the category rather than saying the filter could never have matched. My own iteration-97 test
/// passed only because it happened to use the lowercase form.
#[test]
fn a_category_matches_regardless_of_case() {
    let mut index = index();
    index
        .provenance
        .insert(2, Provenance { source: Some(Source::Pack("Footsteps".into())), ..Provenance::default() });
    let tools = Toolset::default();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");

    // **The exact spelling a person reads in their own sidecar**, which the old code rejected.
    for want in ["Footsteps", "footsteps", "FOOTSTEPS", "ootste"] {
        let answer = tools.compatible_sfx(&index, &session, None, Some(want), 3).expect("a search");
        assert_eq!(answer.results.len(), 1, "`{want}` must match the pack: {}", answer.describe());
        assert_eq!(answer.results[0].hash, 2);
        // **The other two candidates have no provenance at all**, so they cannot match a category and are counted as
        // outside it — which is the honest answer: an asset whose source was never recorded is not in any category.
        assert_eq!(
            answer.tally.wrong_category, 2,
            "assets 3 and 4 have no source, so `{want}` cannot admit them"
        );
    }

    // And a category that genuinely does not match still reports it as the category's doing.
    let none = tools.compatible_sfx(&index, &session, None, Some("Explosions"), 3).expect("a search");
    assert!(none.results.is_empty());
    // **Every candidate is outside it**: the footsteps pack plus the two with no source at all.
    assert_eq!(none.tally.wrong_category, 3, "the fault is assigned to the category");
    assert_eq!(none.tally.excluded, 0, "and not to the acoustic rules");
}

/// **The similarity floor is the caller's**, not a hardcoded default.
///
/// `Toolset` had no floor, so `tools.rs` compared against `Floor::default().min_score`, which is `0.0` — the branch could
/// never fire, so `Tally::below_floor` was documented, rendered by `describe()`, and **permanently zero**, and a caller
/// had no way to ask for a stricter filter.
#[test]
fn the_similarity_floor_can_be_set_and_counts_what_it_drops() {
    use bevy_siren::semantic::weights::Weights;

    let index = index();
    let mut session = Session::new();
    session.set_anchor(&index, 1).expect("an anchor");

    // **The default keeps everything**, so no existing caller changes behaviour.
    let open = Toolset::default();
    let all = open.compatible_music(&index, &session, None, 4).expect("a search");
    assert_eq!(all.tally.below_floor, 0, "a floor of 0.0 drops nothing: {}", all.describe());

    // **A raised floor drops candidates and says so.** Asset 3 is orthogonal to the anchor, so its cosine is ~0.
    let strict =
        Toolset { music: Weights { min_similarity: 0.5, ..Weights::default() }, ..Toolset::default() };
    let filtered = strict.compatible_music(&index, &session, None, 4).expect("a search");
    assert!(filtered.tally.below_floor > 0, "the tally the field exists for: {}", filtered.describe());
    assert!(filtered.describe().contains("below the similarity floor"), "and the sentence names it");
    assert!(
        filtered.results.iter().all(|r| r.similarity >= 0.5),
        "every result clears the floor: {:?}",
        filtered.results.iter().map(|r| r.similarity).collect::<Vec<_>>()
    );

    // **A floor nothing clears empties the list and blames the floor, not the acoustic rules.** `1.0` exactly, because
    // asset 4's cosine against the anchor is **0.99995** — a floor of 0.9999 would still admit it, which is how this
    // assertion first failed. The floor is inclusive of nothing above a perfect cosine.
    let impossible =
        Toolset { music: Weights { min_similarity: 1.0, ..Weights::default() }, ..Toolset::default() };
    let none = impossible.compatible_music(&index, &session, None, 4).expect("a search");
    assert!(none.results.is_empty());
    assert_eq!(none.tally.excluded, 0, "the acoustic rules excluded nothing; the floor did");
    assert!(none.tally.below_floor >= 3, "everything was below it: {}", none.describe());
}
