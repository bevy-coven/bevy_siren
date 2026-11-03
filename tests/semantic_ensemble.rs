//! `#40`'s four criteria, each as a test that fails when the property it names is broken.

#[cfg(feature = "semantic")]
mod ensemble {
    use std::collections::BTreeMap;

    use bevy_siren::model::{Breakdown, Contribution, SIGNALS, Signal};
    use bevy_siren::semantic::ensemble::{agreements, breakdown};
    use bevy_siren::semantic::filter::{Features, evaluate};
    use bevy_siren::semantic::index::{Index, ModelStamp};
    use bevy_siren::semantic::query::Query;
    use bevy_siren::semantic::rank::Floor;
    use bevy_siren::semantic::search::{Ranked, search_ranked};
    use bevy_siren::semantic::weights::Weights;

    /// Features carrying only a noise floor, which is the pair the acoustic signal reads.
    fn with_noise(noise_floor_db: f32) -> Features {
        Features {
            spectral: Some(bevy_siren::semantic::spectral::Spectral {
                centroid_hz: None,
                rolloff_hz: None,
                noise_floor_db,
            }),
            ..Features::default()
        }
    }

    /// A `Weights` where only the signals under test carry weight, so an assertion names one cause.
    fn only(semantic: f32, acoustic: f32, tempo: f32, key: f32) -> Weights {
        Weights { semantic, acoustic, tempo, key, ..Weights::default() }
    }

    /// **Criterion 2: every component is reported with its weight and contribution, and they sum to the score.**
    #[test]
    fn the_contributions_sum_to_the_score() {
        let w = Weights::default();
        let empty = Features::default();
        let judgement = evaluate(&empty, &empty, &w);
        let b = breakdown(0.42, &empty, &empty, &judgement, &w);

        let summed: f32 = SIGNALS.iter().map(|s| b.part(*s).contribution).sum();
        assert!((summed - b.score()).abs() < 1e-6, "the parts must sum to the score");
        assert_eq!(b.parts.len(), 4);

        // Every part carries its weight, and the contribution is `raw * weight` for the ones that counted.
        for s in SIGNALS {
            let p = b.part(s);
            assert!(p.weight >= 0.0, "{} carries a weight", s.name());
            if !p.missing {
                assert!(
                    (p.contribution - p.raw * p.weight).abs() < 1e-6,
                    "{}'s contribution is raw times weight",
                    s.name()
                );
            }
        }
        // The semantic weight is the default's 1.0, and its raw value is the affine map of the cosine.
        assert_eq!(b.part(Signal::Semantic).weight, w.semantic);
        assert!((b.part(Signal::Semantic).raw - 0.71).abs() < 1e-6, "0.42 maps to 0.71");
    }

    /// **Criterion 2, the other half: an absent signal says so rather than contributing a silent zero.**
    #[test]
    fn a_signal_with_no_value_is_missing_and_is_not_a_zero() {
        let w = Weights::default();
        // No tempo, no key, no noise floor: only the semantic signal has anything to say.
        let empty = Features::default();
        let judgement = evaluate(&empty, &empty, &w);
        let b = breakdown(0.2, &empty, &empty, &judgement, &w);

        assert!(!b.part(Signal::Semantic).missing, "semantic always has a value");
        for s in [Signal::Acoustic, Signal::Tempo, Signal::Key] {
            let p = b.part(s);
            assert!(p.missing, "{} had nothing to say, so it must be marked missing", s.name());
            assert_eq!(p.contribution, 0.0);
        }
        let explanation = b.explain();
        assert!(explanation.contains("semantic"), "the explanation names what carried it: {explanation}");
    }

    /// **Criterion 1: the arithmetic is engine-free** — this file is under `tests/`, so it compiles with no `App`, and
    /// the types it uses live in `src/model.rs`, which `tests/all.rs` scans for engine names. That scan is the other half
    /// of this criterion and runs on its own.
    #[test]
    fn the_breakdown_is_usable_with_no_app() {
        let b = Breakdown::new([
            Contribution::scored(Signal::Semantic, 0.8, 1.0),
            Contribution::absent(Signal::Acoustic, 0.5),
            Contribution::absent(Signal::Tempo, 0.35),
            Contribution::absent(Signal::Key, 0.35),
        ]);
        assert!((b.score() - 0.8).abs() < 1e-6);
        assert!(b.shares().is_some());
    }

    /// **Criterion 3: determinism.** The same query on the same index returns the same order and the same scores,
    /// field for field.
    #[test]
    fn the_same_query_returns_identical_order_and_scores() {
        let stamp =
            ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 };
        let mut index = Index::new(stamp.clone());
        let _ = index.insert(1, vec![1.0, 0.0, 0.0]);
        let _ = index.insert(2, vec![0.9, 0.1, 0.0]);
        let _ = index.insert(3, vec![0.0, 1.0, 0.0]);

        let query = Query::new("anything").expect("a query");
        let embedding = vec![1.0f32, 0.0, 0.0];
        let features = BTreeMap::new();
        let w = Weights::default();

        let request = Ranked {
            embedding: &embedding,
            anchor: 1,
            features: &features,
            k: 3,
            floor: Floor::default(),
            weights: &w,
        };
        let first = search_ranked(&index, &stamp, &query, &request).expect("a search");
        let second = search_ranked(&index, &stamp, &query, &request).expect("a search");
        assert_eq!(first, second, "two identical calls must agree field for field");
    }

    /// **Criterion 4: removing a signal changes the order on a fixture where that signal matters.**
    ///
    /// The fixture is built so the acoustic signal reverses what the embedding chose: asset 1 is the nearest to the
    /// query, asset 2 is slightly farther but matches the anchor's noise floor while asset 1 does not. With acoustic
    /// weighted the order flips; with it zeroed the embedding's order stands. If both orders were the same, the signal
    /// would be decorative.
    #[test]
    fn removing_a_signal_that_matters_changes_the_order() {
        let stamp =
            ModelStamp { model: "test".into(), version: "1".into(), sample_rate: 48_000, dimension: 3 };
        let mut index = Index::new(stamp.clone());
        let _ = index.insert(1, vec![1.0, 0.0, 0.0]);
        let _ = index.insert(2, vec![0.99, 0.02, 0.0]);
        // The anchor is a **third** asset, not one of the two being ranked: an anchor that is also a candidate carries
        // its own perfect similarity and would win regardless of the acoustic signal.
        let _ = index.insert(9, vec![0.5, 0.5, 0.0]);

        let query = Query::new("anything").expect("a query");
        let embedding = vec![1.0f32, 0.0, 0.0];
        let anchor = 9u64;
        let mut features = BTreeMap::new();
        // The anchor is quiet; asset 1 is 20 dB noisier, asset 2 matches it.
        features.insert(anchor, with_noise(-80.0));
        features.insert(1, with_noise(-60.0));
        features.insert(2, with_noise(-80.0));
        features.insert(9, with_noise(-80.0));

        let with_acoustic = Weights::default();
        let acoustic_off = only(1.0, 0.0, 0.0, 0.0);

        let with = Ranked {
            embedding: &embedding,
            anchor,
            features: &features,
            k: 3,
            floor: Floor::default(),
            weights: &with_acoustic,
        };
        let off = Ranked {
            embedding: &embedding,
            anchor,
            features: &features,
            k: 3,
            floor: Floor::default(),
            weights: &acoustic_off,
        };
        let ranked = search_ranked(&index, &stamp, &query, &with).expect("a search");
        let plain = search_ranked(&index, &stamp, &query, &off).expect("a search");

        // **Strip the anchor before comparing.** It is its own nearest neighbour and scores a perfect similarity, so it
        // sits first in both runs and says nothing about whether the acoustic signal reordered the candidates.
        let order = |a: &bevy_siren::semantic::search::RankedAnswer| -> Vec<u64> {
            a.matches.iter().map(|m| m.hash).filter(|h| *h != anchor).collect()
        };
        let ranked_order = order(&ranked);
        let plain_order = order(&plain);
        assert_eq!(plain_order, vec![1, 2], "without acoustic weight the embedding's order stands");
        assert_eq!(
            ranked_order,
            vec![2, 1],
            "with acoustic weight the quieter neighbour wins, or it is decorative"
        );
    }

    /// The agreement map is monotonic in the cosine, so weighting cannot invert a similarity order by itself.
    #[test]
    fn the_semantic_map_preserves_order() {
        let w = Weights::default();
        let empty = Features::default();
        let judgement = evaluate(&empty, &empty, &w);
        let mut previous = f32::NEG_INFINITY;
        for cosine in [-1.0, -0.5, 0.0, 0.15, 0.5, 0.9, 1.0] {
            let raw = agreements(cosine, &empty, &empty, &judgement, &w)[Signal::Semantic.index()]
                .expect("a finite cosine has a value");
            assert!(raw > previous, "the map must be strictly increasing at {cosine}");
            previous = raw;
        }
    }

    /// A pair inside the tempo tolerance scores higher than one outside it, and an unmeasured pair is missing.
    #[test]
    fn tempo_agreement_falls_with_distance_and_is_absent_when_unmeasured() {
        let w = Weights::default();
        let empty = Features::default();
        let judgement = evaluate(&empty, &empty, &w);
        let tempo = |bpm: f32| Features {
            tempo: Some(bevy_siren::semantic::tempo::Tempo {
                bpm: Some(bpm),
                confidence: 1.0,
                octave_ambiguous: false,
            }),
            ..Features::default()
        };
        let near = agreements(0.5, &tempo(121.0), &tempo(120.0), &judgement, &w)[Signal::Tempo.index()];
        let far = agreements(0.5, &tempo(126.0), &tempo(120.0), &judgement, &w)[Signal::Tempo.index()];
        assert!(near > far, "a nearer tempo must score higher: {near:?} against {far:?}");
        let absent = agreements(0.5, &empty, &tempo(120.0), &judgement, &w)[Signal::Tempo.index()];
        assert_eq!(absent, None, "an unmeasured tempo is absent, not zero");
    }
}
