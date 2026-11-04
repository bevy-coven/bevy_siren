//! **`#35`'s golden set**: the metrics against orders a person can check by eye, the loader's refusals, and the checked-in set's
//! own invariants.
//!
//! No model, no network, no device. What is tested is the *measuring stick* — which is what decides whether a later ranking change
//! counts as an improvement — and the two properties of the set itself that `#35` makes acceptance criteria: its contents are fixed
//! and versioned, and **every query says why its answer is right**. The benchmark that puts the real model through it
//! (`examples/goldenbench.rs`) needs both and is run by hand.

#![cfg(feature = "semantic")]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bevy_siren::library;
use bevy_siren::semantic::eval::{
    ScoreBand, ScoredCase, centred, floor_curve, score_bands, score_cases, separation,
};
use bevy_siren::semantic::golden::{GoldenProblem, GoldenSet};

fn case(description: &str, right: &[u64], scored: &[(u64, f32)]) -> ScoredCase {
    ScoredCase {
        description: description.to_string(),
        right: right.iter().copied().collect(),
        scored: scored.to_vec(),
    }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// -------------------------------------------------------------------------------------------------------------- the metrics

/// A right answer at rank 9 is not "found" by `k = 3`: the distinction a membership test would lose.
#[test]
fn a_hit_is_a_prefix_not_a_membership_test() {
    let order: Vec<(u64, f32)> = (1..=10u64).map(|h| (h, 1.0 - h as f32 / 100.0)).collect();
    let late = case("late", &[9], &order);
    assert!(!late.hit(3), "rank 9 is not within 3");
    assert!(!late.hit(8));
    assert!(late.hit(9));
    assert_eq!(late.first_right(), Some(9));
    assert!(
        !case("absent", &[99], &order).hit(10),
        "a right answer that is not in the ranking is a miss however large k is"
    );
}

#[test]
fn precision_divides_by_k_even_when_the_library_is_smaller() {
    let two = case("two assets", &[1], &[(1, 0.9), (2, 0.1)]);
    assert!(close(two.precision(10), 0.1), "one right of ten asked for, not one of two returned");
    assert!(close(two.precision(1), 1.0));
    assert_eq!(two.precision(0), 0.0, "k = 0 is not a division");
}

#[test]
fn average_precision_is_the_mean_of_the_precision_at_each_right_answer() {
    // Right answers at ranks 1 and 3: precision 1/1 at the first, 2/3 at the second, over the two there could be.
    let c = case("ap", &[1, 3], &[(1, 0.9), (2, 0.8), (3, 0.7), (4, 0.6), (5, 0.5)]);
    assert!(close(c.average_precision(5), (1.0 + 2.0 / 3.0) / 2.0), "{}", c.average_precision(5));
    assert!(close(c.precision(5), 0.4));
    assert!(close(c.reciprocal_rank(), 1.0));

    // One right answer, at rank 9: 1/9 within ten, nothing within three.
    let order: Vec<(u64, f32)> = (1..=10u64).map(|h| (h, 1.0 - h as f32 / 100.0)).collect();
    let late = case("late", &[9], &order);
    assert!(close(late.average_precision(10), 1.0 / 9.0));
    assert_eq!(late.average_precision(3), 0.0);
    assert!(close(late.reciprocal_rank(), 1.0 / 9.0));
}

/// A query that should find nothing is not a failed query: `score_cases` leaves it out, and `floor_curve` is what it is for.
#[test]
fn queries_that_should_find_nothing_are_not_scored_as_misses() {
    let order: Vec<(u64, f32)> = (1..=10u64).map(|h| (h, 1.0 - h as f32 / 100.0)).collect();
    let cases = [
        case("found", &[1], &order),
        case("late", &[7], &order),
        case("absent", &[99], &order),
        case("nothing should match", &[], &order),
    ];
    let scores = score_cases(&cases);
    assert_eq!(scores.queries, 3, "the empty-right query is not one of the three");
    assert!(close(scores.hit_1, 1.0 / 3.0));
    assert!(close(scores.hit_3, 1.0 / 3.0));
    assert!(close(scores.hit_10, 2.0 / 3.0), "`late` is within ten, `absent` never is");
    assert_eq!(
        scores.missed,
        vec![(String::from("late"), Some(7)), (String::from("absent"), None)],
        "the misses are named, with the rank their first right answer got — or none"
    );
    assert_eq!(score_cases(&[case("only a negative", &[], &order)]).queries, 0);
    assert_eq!(score_cases(&[]).queries, 0);
}

// ---------------------------------------------------------------------------------------------------------- the floor curve

fn floor_fixture() -> Vec<ScoredCase> {
    vec![
        case("a", &[1, 2], &[(1, 0.40), (10, 0.30), (2, 0.20), (11, 0.10)]),
        case("b", &[3], &[(12, 0.35), (3, 0.25), (13, 0.05), (14, -0.10)]),
        case("nothing", &[], &[(15, 0.28), (16, 0.20), (17, 0.0)]),
    ]
}

#[test]
fn a_floor_keeps_what_clears_it_and_silences_what_does_not() {
    let rows = floor_curve(&floor_fixture(), &[-1.0, 0.0, 0.3, 0.5]);
    let (everything, zero, middle, above) = (rows[0], rows[1], rows[2], rows[3]);

    // Below every score: all of it comes back. A's best is right, B's is not.
    assert!(close(everything.top_1_kept, 0.5) && close(everything.hit_3_kept, 1.0));
    assert!(close(everything.returned, 4.0));
    assert!(close(everything.precision.expect("something was returned"), 3.0 / 8.0));
    assert_eq!(everything.silent, Some(0.0), "the query that should find nothing finds three things");

    // At zero the one negative score drops out of B (−0.10) but the 0.0 itself clears a floor of 0.0.
    assert!(close(zero.returned, 3.5), "(4 + 3) / 2");
    assert!(close(zero.precision.expect("returned"), 3.0 / 7.0));
    assert_eq!(zero.silent, Some(0.0));

    // At 0.3: A keeps 0.40 and 0.30, B keeps 0.35 — and B's right answer (0.25) is gone, so B is no longer answered.
    assert!(close(middle.top_1_kept, 0.5));
    assert!(close(middle.hit_3_kept, 0.5), "A's right answer is first and kept; B's is below the floor");
    assert!(close(middle.returned, 1.5));
    assert!(close(middle.precision.expect("returned"), 1.0 / 3.0));
    assert_eq!(middle.silent, Some(1.0), "the negative's best is 0.28, under 0.3");

    // Above every score: nothing is returned, nothing is answered, every negative is silent — and precision is not a number.
    assert_eq!((above.top_1_kept, above.hit_3_kept, above.returned), (0.0, 0.0, 0.0));
    assert_eq!(above.precision, None, "nothing returned has no precision, which is not the same as zero");
    assert_eq!(above.silent, Some(1.0));
}

/// A right answer that ranks first but scores under the floor is **lost**, not kept: the floor is applied before the ranking is read.
#[test]
fn a_right_answer_below_the_floor_is_a_loss_even_when_it_ranked_first() {
    let c = [case("best is right", &[1], &[(1, 0.2), (2, 0.1)])];
    let rows = floor_curve(&c, &[0.15, 0.25]);
    assert_eq!(rows[0].top_1_kept, 1.0);
    assert_eq!(rows[1].top_1_kept, 0.0);
    assert_eq!(rows[1].silent, None, "no query here should find nothing, so there is no share to state");
}

#[test]
fn a_nan_score_clears_no_floor() {
    let c = [case("nan", &[1], &[(1, f32::NAN)])];
    assert_eq!(floor_curve(&c, &[-1.0])[0].returned, 0.0);
}

// ------------------------------------------------------------------------------------------------------------------- bands

#[test]
fn bands_are_quantiles_by_nearest_rank_of_scores_that_occurred() {
    let right: Vec<(u64, f32)> = (0..101u64).map(|i| (1000 + i, 101.0 - i as f32)).collect();
    let mut scored = right.clone();
    scored.extend([(1, 0.5), (2, 0.4), (3, 0.3), (4, 0.2), (5, 0.1)]);
    let right_set: Vec<u64> = right.iter().map(|(h, _)| *h).collect();

    let bands =
        score_bands(&[case("one", &right_set, &scored)]).expect("a positive query with a wrong answer");
    let r = bands.right;
    assert_eq!(
        (r.n, r.min, r.p05, r.p25, r.median, r.p75, r.p95, r.max),
        (101, 1.0, 6.0, 26.0, 51.0, 76.0, 96.0, 101.0)
    );
    let w = bands.wrong;
    assert_eq!(
        (w.n, w.min, w.p05, w.p25, w.median, w.p75, w.p95, w.max),
        (5, 0.1, 0.1, 0.2, 0.3, 0.4, 0.5, 0.5)
    );
    assert_eq!(bands.negative_best, None, "no query here should find nothing");

    let with_negative =
        score_bands(&[case("one", &right_set, &scored), case("none", &[], &[(9, 0.25), (8, 0.1)])])
            .expect("bands");
    assert_eq!(
        with_negative.negative_best.expect("a negative query was given").median,
        0.25,
        "its best score, not its worst"
    );
}

#[test]
fn a_band_ignores_nan_and_an_empty_list_has_none() {
    assert_eq!(ScoreBand::of(vec![]), None);
    assert_eq!(ScoreBand::of(vec![f32::NAN]), None);
    assert_eq!(ScoreBand::of(vec![0.5, f32::NAN, 0.1]).expect("two numbers").n, 2);
    assert!(
        score_bands(&[case("no wrong answer", &[1], &[(1, 0.9)])]).is_none(),
        "nothing to compare the right answers with"
    );
}

// ------------------------------------------------------------------------------------------------- centred, and separation

/// **A query's scores, relative to the query.** Every score less the median of that query's own scores over the library (nearest rank, the
/// rule `ScoreBand` states, so the median is a score that occurred). The order, the hashes, the right answers and the description do not move:
/// only the origin does, which is why the floor curve and the bands apply to centred cases exactly as they are.
#[test]
fn centring_subtracts_the_queries_own_median_and_changes_nothing_else() {
    let raw = case("a", &[1], &[(1, 0.5), (2, 0.3), (3, 0.1)]);
    let out = centred(std::slice::from_ref(&raw));
    assert_eq!(out.len(), 1, "one case in, one out");
    assert_eq!(out[0].description, raw.description);
    assert_eq!(out[0].right, raw.right, "the right answers do not move");
    assert_eq!(
        out[0].scored.iter().map(|(h, _)| *h).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "nor does the order"
    );
    let shifted: Vec<f32> = out[0].scored.iter().map(|(_, s)| *s).collect();
    assert!(
        close(shifted[0], 0.2) && close(shifted[1], 0.0) && close(shifted[2], -0.2),
        "0.5, 0.3 and 0.1 less their median, 0.3: {shifted:?}"
    );
}

/// The median of an even number of scores is the upper middle one — **a score that occurred**, as `ScoreBand` promises — and not the mean of
/// the two in the middle, which no clip scored.
#[test]
fn the_median_of_an_even_count_is_a_score_that_occurred() {
    let out = centred(&[case("a", &[1], &[(1, 0.8), (2, 0.6), (3, 0.2), (4, 0.0)])]);
    let shifted: Vec<f32> = out[0].scored.iter().map(|(_, s)| *s).collect();
    assert!(
        close(shifted[0], 0.2)
            && close(shifted[1], 0.0)
            && close(shifted[2], -0.4)
            && close(shifted[3], -0.6),
        "the median of 0.8, 0.6, 0.2, 0.0 is 0.6 by nearest rank: {shifted:?}"
    );
}

#[test]
fn centring_ignores_nan_when_finding_the_median_and_leaves_an_empty_case_alone() {
    let out = centred(&[case("a", &[1], &[(1, 0.5), (3, 0.1), (2, f32::NAN)]), case("none", &[], &[])]);
    let shifted: Vec<f32> = out[0].scored.iter().map(|(_, s)| *s).collect();
    assert!(
        close(shifted[0], 0.0) && close(shifted[1], -0.4) && shifted[2].is_nan(),
        "the median of 0.5 and 0.1 is 0.5 by nearest rank, and a NaN stays one: {shifted:?}"
    );
    assert!(out[1].scored.is_empty(), "nothing to centre, and no panic");
}

/// `positive` and `negative` are the top scores of answerable and unanswerable queries; each has a second, lower result so the list is a ranking.
fn tops(positive: &[f32], negative: &[f32]) -> Vec<ScoredCase> {
    let mut cases = Vec::new();
    for (i, top) in positive.iter().enumerate() {
        let hash = 1000 + i as u64;
        cases.push(case(&format!("p{i}"), &[hash], &[(hash, *top), (1, *top - 0.2)]));
    }
    for (i, top) in negative.iter().enumerate() {
        cases.push(case(&format!("n{i}"), &[], &[(2000 + i as u64, *top), (2, *top - 0.2)]));
    }
    cases
}

/// **Separation: can a floor on the top score tell a description the library can answer from one it cannot?** The AUC is the chance that a
/// random answerable query's top score beats a random unanswerable query's — 0.5 is a coin, 1.0 is a floor that exists.
#[test]
fn the_auc_is_the_chance_an_answerable_querys_top_score_beats_an_unanswerable_ones() {
    let s = separation(&tops(&[0.6, 0.5, 0.4], &[0.45, 0.3])).expect("both kinds of query");
    assert_eq!((s.answerable, s.unanswerable), (3, 2));
    assert!((s.auc - 5.0 / 6.0).abs() < 1e-12, "five of the six pairs: {}", s.auc);
    assert_eq!((s.weakest_answerable, s.strongest_unanswerable), (0.4, 0.45));
    assert!(!s.floor_exists(), "0.4 < 0.45: no floor keeps every answerable query and silences every other");

    let clean = separation(&tops(&[0.6, 0.5], &[0.3, 0.2])).expect("both kinds of query");
    assert_eq!(clean.auc, 1.0);
    assert!(clean.floor_exists(), "0.5 > 0.3: any floor between them separates the two groups");
}

#[test]
fn a_tie_counts_half() {
    let s = separation(&tops(&[0.6, 0.5, 0.4, 0.45], &[0.45, 0.3])).expect("both kinds of query");
    assert!((s.auc - 6.5 / 8.0).abs() < 1e-12, "six and a half of eight pairs: {}", s.auc);
}

#[test]
fn separation_needs_both_kinds_of_query_and_a_top_score_to_read() {
    assert!(separation(&tops(&[0.6], &[])).is_none(), "nothing unanswerable to part it from");
    assert!(separation(&tops(&[], &[0.3])).is_none(), "nothing answerable");
    assert!(separation(&[]).is_none());
    let mut cases = tops(&[0.6], &[0.3]);
    cases.push(case("an answerable query with no ranking", &[1], &[]));
    cases.push(case("a query whose top score is NaN", &[], &[(9, f32::NAN)]));
    let s = separation(&cases).expect("the two with a top score");
    assert_eq!((s.answerable, s.unanswerable), (1, 1), "a query with no top score to read is not counted");
}

/// What a floor reads is the **top score**, whoever it belongs to: an answerable query whose best result is *wrong* still has that top score.
#[test]
fn the_top_score_is_read_whether_or_not_it_is_a_right_answer() {
    let s = separation(&[case("p", &[2], &[(1, 0.9), (2, 0.1)]), case("n", &[], &[(3, 0.5)])]).expect("both");
    assert_eq!(s.auc, 1.0);
    assert_eq!(s.weakest_answerable, 0.9, "the wrong result's 0.9, not the right one's 0.1");
}

/// **Why centre.** A description that every clip matches a little can out-score one that has a real answer — its *top* score is high because all
/// its scores are. Subtracting each query's own median removes that offset, and the two kinds of query part.
#[test]
fn centring_parts_a_query_whose_scores_are_all_high_from_one_with_a_real_answer() {
    let answerable = case("a", &[1], &[(1, 0.30), (2, 0.10), (3, 0.00)]);
    let unanswerable = case("n", &[], &[(4, 0.35), (5, 0.34), (6, 0.33)]);
    let raw = separation(&[answerable.clone(), unanswerable.clone()]).expect("both");
    assert_eq!(raw.auc, 0.0, "the unanswerable query's top score, 0.35, is the higher");
    let relative = separation(&centred(&[answerable, unanswerable])).expect("both");
    assert_eq!(relative.auc, 1.0, "0.20 above its median against 0.01 above its own");
    assert!(relative.floor_exists());
}

// ---------------------------------------------------------------------------------------------------------------- loading

const LABELS: &str = "file,class\na/dog1.wav,dog\na/dog2.wav,dog\nb/cat1.wav,cat\n";
const QUERIES: &str = "target\tclass\tdescription\trationale\n0\tdog\ta dog barking\tthe dog class\n1\tcat\ta cat meowing\tthe cat class\n-\t-\ta violin\tno violin here\n";

#[test]
fn a_set_loads_and_a_dash_is_a_query_that_should_find_nothing() {
    let set = GoldenSet::parse(LABELS, QUERIES).expect("a valid set");
    assert_eq!(set.labelled(), 3);
    assert_eq!(set.classes(), BTreeSet::from(["cat", "dog"]));
    let classes: Vec<Option<&str>> = set.queries.iter().map(|q| q.class.as_deref()).collect();
    assert_eq!(classes, vec![Some("dog"), Some("cat"), None]);
    assert_eq!(set.queries[2].rationale, "no violin here");
}

/// Columns are named by the header, not by position, so the dataset's own CSV is read as it is shipped.
#[test]
fn the_datasets_own_csv_spelling_reads_the_same() {
    let theirs = "filename,fold,target,category,esc10,src_file,take\n\
                  a/dog1.wav,1,0,dog,True,100032,A\na/dog2.wav,1,0,dog,False,100033,A\nb/cat1.wav,2,5,cat,False,7,A\n";
    assert_eq!(GoldenSet::parse(theirs, QUERIES), GoldenSet::parse(LABELS, QUERIES));
}

fn refused(labels: &str, queries: &str) -> String {
    GoldenSet::parse(labels, queries).expect_err("a refusal").to_string()
}

/// Each refusal names the line or the thing that is wrong, because a golden set with a mistake in it measures the mistake.
#[test]
fn a_mistake_in_the_queries_is_refused_by_line() {
    let no_reason = "class\tdescription\trationale\ndog\ta dog barking\t\n";
    let message = refused(LABELS, no_reason);
    assert!(
        message.contains("queries, line 2")
            && message.contains("a dog barking")
            && message.contains("no rationale"),
        "{message}"
    );

    // Comments and blank lines are skipped, but they still count: the line a person goes to is the file's own line.
    let after_comments = "# a comment\n\nclass\tdescription\trationale\n# another\ndog\ta dog barking\t\n";
    assert!(refused(LABELS, after_comments).contains("line 5"), "{}", refused(LABELS, after_comments));

    let twice = "class\tdescription\trationale\ndog\tA Dog Barking\twhy\ncat\ta dog barking\twhy\n";
    let message = refused(LABELS, twice);
    assert!(
        message.contains("line 3")
            && message.contains("repeats line 2")
            && message.contains("keyed by description"),
        "{message}"
    );

    let no_class = "class\tdescription\trationale\n\ta dog barking\twhy\n";
    assert!(refused(LABELS, no_class).contains("write `-`"), "{}", refused(LABELS, no_class));

    let no_description = "class\tdescription\trationale\ndog\t\twhy\n";
    assert!(refused(LABELS, no_description).contains("no description"));

    let no_header = "dog\ta dog barking\twhy\n";
    assert!(refused(LABELS, no_header).contains("header must name `class`, `description` and `rationale`"));
    assert!(refused(LABELS, "# only a comment\n").contains("no header"));
}

#[test]
fn a_query_for_a_class_no_file_is_in_is_refused_rather_than_blamed_on_the_ranking() {
    let queries = "class\tdescription\trationale\nfrog\ta frog croaking\tthe frog class\n";
    let problem = GoldenSet::parse(LABELS, queries).expect_err("no file is a frog");
    assert_eq!(
        problem,
        GoldenProblem::NoLabelledFile {
            class: String::from("frog"),
            description: String::from("a frog croaking")
        }
    );
    assert!(problem.to_string().contains("always miss"), "{problem}");
}

#[test]
fn a_mistake_in_the_labels_is_refused_by_line() {
    assert!(
        refused("name,kind\nx.wav,dog\n", QUERIES).contains("no `file` column"),
        "{}",
        refused("name,kind\nx.wav,dog\n", QUERIES)
    );
    let twice = "file,class\nx.wav,dog\nx.wav,cat\n";
    let message = refused(twice, QUERIES);
    assert!(message.contains("labels, line 3") && message.contains("labelled twice"), "{message}");
    assert!(refused("file,class\nx.wav,\n", QUERIES).contains("no file or no class"));
    assert!(refused("", QUERIES).contains("empty"));
}

// ----------------------------------------------------------------------------------------------- against a library on disk

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Scratch {
    fn new(name: &str) -> Self {
        // One directory per call, whatever the label (see `semantic_indexer.rs`: two tests that shared a label raced for one directory).
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("bevy_siren_golden_{name}_{}_{unique}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch");
        Scratch(path)
    }

    fn wav(&self, name: &str, seed: u32) {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("parent");
        std::fs::write(path, wav_bytes(seed)).expect("write");
    }
}

/// A valid mono 16-bit 48 kHz WAV; a different `seed` is a different file and so a different content hash.
fn wav_bytes(seed: u32) -> Vec<u8> {
    let rate = 48_000u32;
    let data: Vec<u8> = (0..2_400)
        .map(|i| {
            let t = (i as f32 + seed as f32 * 7.0) / rate as f32;
            ((t * 440.0 * std::f32::consts::TAU).sin() * 8_000.0) as i16
        })
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// The right answers of a class are the assets whose files are labelled with it — by hash, so the set survives a rename.
#[test]
fn right_answers_are_found_by_content_and_a_partial_or_padded_library_is_named() {
    let dir = Scratch::new("library");
    dir.wav("a/dog1.wav", 1);
    dir.wav("a/dog2.wav", 2);
    dir.wav("b/cat1.wav", 3);
    dir.wav("b/extra.wav", 4); // in the folder, labelled by nobody
    let library = library::scan(&dir.0);
    let labels = "file,class\na/dog1.wav,dog\na/dog2.wav,dog\nb/cat1.wav,cat\nb/missing.wav,cat\n";
    let set = GoldenSet::parse(labels, QUERIES).expect("a valid set");

    let by_class = set.right_answers(&library);
    assert_eq!(by_class["dog"].len(), 2);
    assert_eq!(
        by_class["cat"].len(),
        1,
        "`missing.wav` is labelled and not here, so it is not a right answer"
    );
    assert_eq!(
        set.absent(&library),
        vec![String::from("b/missing.wav")],
        "a partial copy is named, not measured"
    );
    assert_eq!(
        set.unlabelled(&library),
        vec![Path::new("b/extra.wav").to_path_buf()],
        "a stray file is named, not ranked"
    );
    assert_eq!(set.label_of(Path::new("a/dog1.wav")), Some("dog"));
    assert_eq!(set.label_of(Path::new("a/unknown.wav")), None);
}

/// A labels file that lists bare file names labels the same library, wherever the files sit.
#[test]
fn a_bare_file_name_labels_a_file_in_any_folder() {
    let dir = Scratch::new("bare");
    dir.wav("deep/er/dog1.wav", 1);
    let library = library::scan(&dir.0);
    let only_dogs = "class\tdescription\trationale\ndog\ta dog barking\tthe dog class\n";
    let set = GoldenSet::parse("filename,category\ndog1.wav,dog\n", only_dogs).expect("a valid set");
    assert_eq!(set.right_answers(&library)["dog"].len(), 1);
    assert!(set.absent(&library).is_empty() && set.unlabelled(&library).is_empty());
}

/// Two paths, one content: the asset is a right answer for every class a path of it is labelled with, and counted once.
#[test]
fn one_content_at_two_labelled_paths_is_one_asset() {
    let dir = Scratch::new("twice");
    dir.wav("a/dog.wav", 1);
    dir.wav("b/dog-copy.wav", 1);
    let library = library::scan(&dir.0);
    let set =
        GoldenSet::parse("file,class\na/dog.wav,dog\nb/dog-copy.wav,cat\n", QUERIES).expect("a valid set");
    let by_class = set.right_answers(&library);
    assert_eq!(by_class["dog"], by_class["cat"], "the same bytes are the right answer for both classes");
    assert_eq!(by_class["dog"].len(), 1);
}

// ------------------------------------------------------------------------------------------------- the set that is checked in

fn repo_file(path: &str) -> String {
    std::fs::read_to_string(format!("{}/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// **`#35`, criterion 1: the contents are fixed and versioned.** The manifest is the set's identity — one SHA-256 per file, at a
/// named commit — so a download that differs in any byte is refused by `tools/fetch_esc50.py`. This asserts the manifest is the
/// set it says: 2 000 clips, 40 to each of 50 classes, plus the dataset's own labels and its per-clip licences.
#[test]
fn the_manifest_pins_two_thousand_clips_forty_to_each_of_fifty_classes() {
    let manifest = repo_file("golden/esc50/manifest.sha256");
    let commit = manifest
        .lines()
        .find_map(|line| line.strip_prefix("# commit:"))
        .map(str::trim)
        .expect("the manifest names the commit it pins");
    assert!(
        commit.len() == 40 && commit.bytes().all(|b| b.is_ascii_hexdigit()),
        "a full commit sha, not a branch: {commit}"
    );

    let mut per_target: BTreeMap<u32, usize> = BTreeMap::new();
    let mut others = BTreeSet::new();
    for line in manifest.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (digest, path) = line.split_once("  ").unwrap_or_else(|| panic!("`<sha256>  <path>`: {line}"));
        assert!(
            digest.len() == 64 && digest.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "a lowercase SHA-256, not a shorter or weaker hash: {line}"
        );
        match path.strip_prefix("audio/").and_then(|name| name.strip_suffix(".wav")) {
            Some(name) => {
                let target = name
                    .rsplit('-')
                    .next()
                    .and_then(|t| t.parse::<u32>().ok())
                    .unwrap_or_else(|| panic!("a clip's name ends in its class number: {path}"));
                *per_target.entry(target).or_default() += 1;
            }
            None => {
                others.insert(path.to_string());
            }
        }
    }
    assert_eq!(
        per_target.keys().copied().collect::<Vec<_>>(),
        (0..50).collect::<Vec<_>>(),
        "classes 0 to 49"
    );
    assert!(per_target.values().all(|&n| n == 40), "forty clips to a class: {per_target:?}");
    assert_eq!(others, BTreeSet::from([String::from("LICENSE"), String::from("meta/esc50.csv")]));
}

/// **`#35`, criterion 2: every text query has an expected answer and the rationale in one line** — in each of the two checked-in
/// query files: the maintainer's own wording, and a paraphrase that avoids the class words. Built from the manifest, so no audio is
/// needed: the labels are the class numbers the file names carry, spelled with the class names the queries give.
#[test]
fn every_checked_in_query_names_its_answer_and_says_why_and_every_class_is_asked_for() {
    let manifest = repo_file("golden/esc50/manifest.sha256");
    let mut spelled: Vec<BTreeMap<u32, String>> = Vec::new();

    for file in ["golden/esc50/queries.tsv", "golden/esc50/queries-paraphrase.tsv"] {
        let queries = repo_file(file);

        // Which class name each number carries, from the queries' own `target` and `class` columns.
        let mut names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
        for line in queries.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#')).skip(1) {
            let cells: Vec<&str> = line.split('\t').collect();
            if let Ok(target) = cells[0].parse::<u32>() {
                names.entry(target).or_default().insert(cells[1].to_string());
            }
        }
        assert_eq!(
            names.keys().copied().collect::<Vec<_>>(),
            (0..50).collect::<Vec<_>>(),
            "{file}: a query for every one of the 50 classes"
        );
        assert!(names.values().all(|n| n.len() == 1), "{file}: one name to a number: {names:?}");

        let labels: String = std::iter::once(String::from("file,class"))
            .chain(manifest.lines().filter_map(|line| {
                let path = line.split_once("  ")?.1;
                let name = path.strip_prefix("audio/")?.strip_suffix(".wav")?;
                let target: u32 = name.rsplit('-').next()?.parse().ok()?;
                Some(format!("{path},{}", names[&target].iter().next()?))
            }))
            .collect::<Vec<_>>()
            .join("\n");

        // `parse` is where "a rationale on every row, descriptions unique, every class has files" is enforced.
        let set = GoldenSet::parse(&labels, &queries).unwrap_or_else(|e| panic!("{file} does not load: {e}"));
        assert_eq!(set.labelled(), 2_000, "{file}");
        assert_eq!(set.classes().len(), 50, "{file}");
        let asked: BTreeSet<&str> = set.queries.iter().filter_map(|q| q.class.as_deref()).collect();
        assert_eq!(asked, set.classes(), "{file}: every class is asked for by at least one description");
        let negatives = set.queries.iter().filter(|q| q.class.is_none()).count();
        assert!(
            negatives >= 10,
            "{file}: queries that should find nothing are how a floor is shown to work: {negatives}"
        );
        assert!(
            set.queries.iter().all(|q| !q.rationale.is_empty() && !q.rationale.contains('\t')),
            "{file}: one line each"
        );
        spelled.push(
            names.into_iter().filter_map(|(target, n)| Some((target, n.into_iter().next()?))).collect(),
        );
    }
    assert_eq!(spelled[0], spelled[1], "the two files call every class number by the same name");
}

// ------------------------------------------------------------------------------------------- the second library, checked in

fn is_sha256(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Each class and the number of files in it: what `tools/fetch_kenney.py` asserts and the packs' own pages support — *"23 card handling
/// sounds, 19 chip handling and 12 dice handling"*, *"9 lasers, 15 phasers, 12 power-ups, 6 zaps"*, *"17 cues each in 8-bit, hit, pizzicato,
/// sax and steel groups"*. Spelled out here as well, so that changing a label is a change in two places.
const KENNEY: [(&str, usize); 29] = [
    ("footstep", 10),
    ("door", 6),
    ("book", 8),
    ("cloth", 4),
    ("knife", 5),
    ("metal_pot", 3),
    ("leather", 3),
    ("playing_cards", 23),
    ("poker_chips", 19),
    ("dice", 12),
    ("chiptune", 17),
    ("saxophone", 17),
    ("steel_drum", 17),
    ("pizzicato", 17),
    ("mouse_click", 12),
    ("error_beep", 8),
    ("confirmation_chime", 4),
    ("scrolling", 5),
    ("laser", 24),
    ("power_up", 12),
    ("electric_zap", 6),
    ("phaser", 15),
    ("explosion", 7),
    ("force_field", 5),
    ("sci_fi_door", 6),
    ("engine", 24),
    ("thruster", 5),
    ("computer_noise", 4),
    ("metal_impact", 5),
];

/// **`#35`, criterion 1, for the second library: the contents are fixed and versioned.** One SHA-256 per file and per archive, a version, and the
/// licence text of each pack beside its audio — and the labels name exactly the audio the manifest pins, in the classes the table above lists.
#[test]
fn the_kenney_manifest_pins_seven_archives_and_303_files_in_29_classes() {
    let manifest = repo_file("golden/kenney/manifest.sha256");
    assert_eq!(
        manifest.lines().find_map(|l| l.strip_prefix("# version:")).map(str::trim),
        Some("v1"),
        "a set with no commit pins its contents by a version, which the benchmark's identity reads"
    );
    let packs: Vec<Vec<&str>> = manifest
        .lines()
        .filter_map(|l| l.strip_prefix("# pack:"))
        .map(|l| l.split_whitespace().collect())
        .collect();
    assert_eq!(packs.len(), 7, "seven archives");
    for pack in &packs {
        assert!(
            pack.len() == 3 && is_sha256(pack[1]) && pack[2].ends_with(".zip"),
            "`<pack> <sha256> <archive>`: {pack:?}"
        );
    }

    let (mut audio, mut licences) = (BTreeMap::new(), BTreeSet::new());
    for line in manifest.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let (digest, path) = line.split_once("  ").unwrap_or_else(|| panic!("`<sha256>  <path>`: {line}"));
        assert!(is_sha256(digest), "a lowercase SHA-256, not a shorter or weaker hash: {line}");
        if path.starts_with("audio/") {
            audio.insert(path.to_string(), digest.to_string());
        } else {
            licences.insert(path.to_string());
        }
    }
    assert_eq!(audio.len(), 303);
    assert_eq!(licences.len(), 7, "each pack's own licence text is pinned beside its audio");
    let distinct: BTreeSet<&String> = audio.values().collect();
    assert_eq!(
        distinct.len(),
        303,
        "no two paths hold the same bytes: the library would hold one asset for two rows"
    );

    let labels = repo_file("golden/kenney/labels.csv");
    let (mut by_class, mut labelled): (BTreeMap<String, usize>, BTreeSet<String>) = Default::default();
    for line in labels.lines().skip(1) {
        let (file, class) = line.split_once(',').unwrap_or_else(|| panic!("`file,class`: {line}"));
        labelled.insert(file.to_string());
        *by_class.entry(class.to_string()).or_default() += 1;
    }
    assert_eq!(
        labelled,
        audio.keys().cloned().collect::<BTreeSet<_>>(),
        "the labels name exactly the files the manifest pins"
    );
    let expected: BTreeMap<String, usize> = KENNEY.iter().map(|(class, n)| (class.to_string(), *n)).collect();
    assert_eq!(by_class, expected);
}

/// **`#35`, criterion 2, for the second library: every text query has an expected answer and the rationale in one line**, in each of the two
/// files — and the two ask the same eighteen questions of sounds that are not here.
#[test]
fn every_kenney_query_names_its_answer_and_says_why_in_both_wordings() {
    let labels = repo_file("golden/kenney/labels.csv");
    let mut negatives: Vec<BTreeSet<String>> = Vec::new();
    for file in ["golden/kenney/queries.tsv", "golden/kenney/queries-paraphrase.tsv"] {
        // `parse` is where a rationale on every row, unique descriptions and a labelled file in every asked-for class are enforced.
        let set = GoldenSet::parse(&labels, &repo_file(file))
            .unwrap_or_else(|e| panic!("{file} does not load: {e}"));
        assert_eq!((set.labelled(), set.classes().len()), (303, 29), "{file}");
        let asked: BTreeSet<&str> = set.queries.iter().filter_map(|q| q.class.as_deref()).collect();
        assert_eq!(asked, set.classes(), "{file}: every class is asked for");
        assert_eq!(
            set.queries.iter().filter(|q| q.class.is_some()).count(),
            29,
            "{file}: one description for each class"
        );
        assert!(
            set.queries.iter().all(|q| !q.rationale.is_empty() && !q.rationale.contains('\t')),
            "{file}: one line each"
        );
        let nothing: BTreeSet<String> =
            set.queries.iter().filter(|q| q.class.is_none()).map(|q| q.description.clone()).collect();
        assert_eq!(
            nothing.len(),
            18,
            "{file}: queries that should find nothing are how a floor is shown to work"
        );
        negatives.push(nothing);
    }
    assert_eq!(
        negatives[0], negatives[1],
        "the same eighteen descriptions of sounds that are not here, asked both ways"
    );
}

/// **The point of the second file, enforced rather than hoped for**: a stranger's wording uses none of its class's own words — the parts of a
/// class name of four letters or more — and is not the maintainer's wording again. (ESC-50's paraphrase file was written to the same rule by
/// hand; this library's is held to it.)
#[test]
fn a_strangers_wording_uses_none_of_its_classs_own_words_and_is_not_the_maintainers() {
    let labels = repo_file("golden/kenney/labels.csv");
    let maintainer = GoldenSet::parse(&labels, &repo_file("golden/kenney/queries.tsv")).expect("loads");
    let stranger =
        GoldenSet::parse(&labels, &repo_file("golden/kenney/queries-paraphrase.tsv")).expect("loads");
    let first: BTreeMap<&str, &str> = maintainer
        .queries
        .iter()
        .filter_map(|q| Some((q.class.as_deref()?, q.description.as_str())))
        .collect();
    for query in stranger.queries.iter().filter(|q| q.class.is_some()) {
        let class = query.class.as_deref().expect("a positive query");
        let words = query.description.to_lowercase();
        for part in class.split('_').filter(|part| part.len() >= 4) {
            assert!(
                !words.contains(part),
                "`{}` asks for `{class}` and contains its own word `{part}`",
                query.description
            );
        }
        assert_ne!(
            first[class], query.description,
            "`{class}`: a second wording that repeats the first is not a second wording"
        );
    }
}

/// **`goldenbench --set <name>` reads `golden/<name>/…` and tells a person to run `tools/fetch_<name>.py`**, so a set directory without any
/// of them is a benchmark that refuses to start or points at nothing; and a set with no baseline has nothing to regress against.
#[test]
fn every_golden_set_has_what_the_benchmark_and_its_fetch_command_need() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sets = Vec::new();
    for entry in std::fs::read_dir(root.join("golden")).expect("golden/") {
        let entry = entry.expect("an entry");
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        for needed in ["manifest.sha256", "queries.tsv", "baseline.txt"] {
            assert!(
                entry.path().join(needed).is_file(),
                "golden/{name}/{needed} is what `goldenbench --set {name}` reads"
            );
        }
        assert!(
            root.join(format!("tools/fetch_{name}.py")).is_file(),
            "`tools/fetch_{name}.py` is the command the benchmark names"
        );
        let manifest = std::fs::read_to_string(entry.path().join("manifest.sha256")).expect("the manifest");
        assert!(
            manifest.lines().any(|l| l.starts_with("# commit:") || l.starts_with("# version:")),
            "golden/{name}/manifest.sha256 pins its contents by a commit or a version, which the benchmark's identity reads"
        );
        sets.push(name);
    }
    assert!(
        sets.iter().any(|s| s == "esc50") && sets.iter().any(|s| s == "kenney"),
        "both sets were seen: {sets:?}"
    );
}
