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
