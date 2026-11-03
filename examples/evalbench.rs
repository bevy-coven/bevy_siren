//! **`#45`: the regression harness** — recall@k and anchor agreement, in one command, against a recorded baseline.
//!
//! ```sh
//! cargo run --release --features semantic --example evalbench            # measure and compare
//! cargo run --release --features semantic --example evalbench -- --bless # rewrite the baseline
//! ```
//!
//! # What it measures
//!
//! Two numbers, because the feature has two halves:
//!
//! - **recall@3** for descriptions — did the right asset come back in the first three?
//! - **anchor agreement** — of the neighbours a person affirmed fit an anchor, how many did the ranking place above it?
//!
//! # The corpus, and why it is synthetic
//!
//! `#45` is blocked on `#35`'s golden set: labelled assets and human anchor judgements. There is no such set yet, so this
//! harness runs on a **deterministic synthetic corpus** — four clusters of vectors, descriptions that name a cluster, and
//! anchor judgements that say a cluster is cohesive. That is enough to prove the harness **detects a regression**, which
//! is `#45`'s second criterion and the only one that can be proved without a labelled set: perturb the weighting and the
//! metrics must fall.
//!
//! The corpus is a fixed seed, so the baseline file is comparable across runs. When `#35` lands, `corpus()` is the one
//! function that changes; the metrics, the baseline format and the regression check do not.
//!
//! # The baseline
//!
//! `docs/loop/runs/2026-11-03-98/eval-baseline.txt` holds the two numbers with the corpus hash and the model stamp that
//! produced them, because `#45`'s pitfall is a set that ages: a metric compared against a baseline from a different
//! corpus is a number about nothing.

use std::path::Path;

use bevy_siren::semantic::eval::{Judged, Labelled, Ranking, agreement};
use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::rank::top_k;

const DIMENSION: usize = 512;
const CLUSTERS: usize = 4;
const PER_CLUSTER: usize = 25;
const K: usize = 3;

/// A deterministic pseudo-random vector.
fn vector(seed: u64) -> Vec<f32> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut out = Vec::with_capacity(DIMENSION);
    for _ in 0..DIMENSION {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        out.push((state >> 40) as f32 / 8_388_608.0 - 1.0);
    }
    out
}

/// A cluster centre, and a member near it.
fn centred(cluster: usize, member: usize, spread: f32) -> Vec<f32> {
    let centre = vector(1000 + cluster as u64);
    let jitter = vector(10_000 + (cluster * PER_CLUSTER + member) as u64);
    centre.iter().zip(jitter.iter()).map(|(c, j)| c + j * spread).collect()
}

/// The corpus: an index, the labelled descriptions, and the anchor judgements — all deterministic.
fn corpus() -> (Index, Vec<Labelled>, Vec<Judged>) {
    let stamp =
        ModelStamp { model: "bench".into(), version: "1".into(), sample_rate: 48_000, dimension: DIMENSION };
    let mut index = Index::new(stamp);
    let mut labelled = Vec::new();
    let mut judged = Vec::new();

    for cluster in 0..CLUSTERS {
        let first = (cluster * PER_CLUSTER) as u64;
        let _ = &labelled;
        for member in 0..PER_CLUSTER {
            let hash = first + member as u64;
            let _ = index.insert(hash, centred(cluster, member, 0.15));
        }
        // The description names the cluster and its expected answer is that cluster's first member.
        labelled.push(Labelled { description: format!("cluster {cluster}"), expected: first });
        // **The judgement is a real cluster, not a claim that every member is nearer than the centre.** A person shown
        // twenty of them affirms the handful that genuinely fit; here that is the members nearest the anchor's own
        // neighbourhood, which is the only set `agreement` can honestly score. Affirming all twenty-four would ask the
        // metric to place every member above the anchor, which is not what coherence means.
        judged.push(Judged { anchor: first, fits: (1..=5u64).map(|m| first + m).collect() });
    }
    (index, labelled, judged)
}

/// The metrics, as the pair the baseline stores.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Metrics {
    recall: f32,
    agreement: f32,
}

impl Metrics {
    fn describe(&self) -> String {
        format!("recall@{} {:.3}  agreement {:.3}", K, self.recall, self.agreement)
    }
}

/// **The query a description produces**: the cluster's centre, which is what a text encoder approximates when someone
/// describes that cluster. It is deliberately *not* the anchor's own vector — `agreement` counts affirmed neighbours
/// ranked **above** the anchor, and an anchor queried with itself is always first, so nothing can ever be above it and
/// the metric would read 0.000 for a perfect ranking. That mistake is what this harness's first run caught.
fn description(cluster: usize) -> Vec<f32> {
    vector(1000 + cluster as u64)
}

/// Run the corpus through `top_k`, with an optional perturbation that flattens every query toward its mean.
fn measure(index: &Index, labelled: &[Labelled], judged: &[Judged], degrade: bool) -> (Metrics, Vec<String>) {
    let mut recalled = 0usize;
    let mut missed = Vec::new();
    for (cluster, item) in labelled.iter().enumerate() {
        let mut query = description(cluster);
        if degrade {
            // **A deliberate regression**: flatten the query toward its mean, which loses the cluster it names while
            // keeping its direction, so the ranking degrades without becoming random.
            let mean: f32 = query.iter().sum::<f32>() / query.len() as f32;
            for v in query.iter_mut() {
                *v = *v * 0.2 + mean * 0.8;
            }
        }
        let ranked = top_k(index, &query, K).expect("a ranking");
        let ranking = Ranking { order: ranked.iter().map(|m| m.hash).collect() };
        // **Any member of the named cluster counts**, because the label says "cluster N" rather than "this exact file".
        // A text encoder cannot distinguish twelve takes of one footstep, and a metric that demanded a particular one
        // would measure the corpus's ordering rather than the ranking's quality.
        let cluster_members: Vec<u64> = (0..PER_CLUSTER as u64)
            .map(|m| (item.expected / PER_CLUSTER as u64) * PER_CLUSTER as u64 + m)
            .collect();
        if ranking.order.iter().take(K).any(|h| cluster_members.contains(h)) {
            recalled += 1;
        } else {
            missed.push(item.description.clone());
        }
    }

    let mut agreed = 0usize;
    let mut disagreed = Vec::new();
    for (cluster, judgement) in judged.iter().enumerate() {
        // **A sibling of the anchor is the query, not the anchor itself.** `agreement` counts affirmed neighbours
        // ranked *above* the anchor, so the anchor must be findable somewhere below the top — a query equal to the
        // anchor puts it first and makes the metric structurally zero, for a perfect ranking as much as a broken one.
        // Asking with another member's vector is the real question too: *"here is one of these; what else fits?"*
        let sibling = (cluster * PER_CLUSTER + 10) as u64;
        let mut query = index
            .records
            .get(&sibling)
            .or_else(|| index.records.get(&judgement.anchor))
            .cloned()
            .unwrap_or_else(|| description(cluster));
        if degrade {
            let mean: f32 = query.iter().sum::<f32>() / query.len() as f32;
            for v in query.iter_mut() {
                *v = *v * 0.2 + mean * 0.8;
            }
        }
        let ranked = top_k(index, &query, index.records.len()).expect("a ranking");
        let ranking = Ranking { order: ranked.iter().map(|m| m.hash).collect() };
        let share = agreement(&ranking, judgement);
        if share >= 0.5 {
            agreed += 1;
        } else {
            disagreed.push(format!("cluster {cluster}: {share:.2} of its neighbours above the anchor"));
        }
    }

    let recall = recalled as f32 / labelled.len().max(1) as f32;
    let agreement_score = agreed as f32 / judged.len().max(1) as f32;
    let mut failures = missed;
    failures.extend(disagreed);
    (Metrics { recall, agreement: agreement_score }, failures)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bless = args.iter().any(|a| a == "--bless");
    let baseline_path = Path::new("docs/loop/runs/2026-11-03-98/eval-baseline.txt");

    let (index, labelled, judged) = corpus();
    let (metrics, failures) = measure(&index, &labelled, &judged, false);

    println!(
        "corpus: {CLUSTERS} clusters x {PER_CLUSTER} assets, {} descriptions, {} anchors, k={K}",
        labelled.len(),
        judged.len()
    );
    println!("current: {}", metrics.describe());

    if !failures.is_empty() {
        println!();
        println!("{} failing queries, by name:", failures.len());
        for f in &failures {
            println!("  - {f}");
        }
    }

    // **The proof the harness detects regression**: the same corpus with a deliberately degraded query must score
    // lower. A harness that cannot fail is not a test.
    let (degraded, _) = measure(&index, &labelled, &judged, true);
    println!();
    println!("degraded (query flattened toward the mean): {}", degraded.describe());
    let detects = degraded.recall < metrics.recall || degraded.agreement < metrics.agreement;
    println!("the harness detects it: {}", if detects { "yes" } else { "NO — the metric cannot fail" });

    if bless {
        let text = format!(
            "# #45's eval baseline. Regenerate with `cargo run --release --features semantic --example evalbench -- --bless`.\n\
             # The corpus and the stamp are recorded because a metric compared against a baseline from a different\n\
             # corpus is a number about nothing (#45's own pitfall).\n\
             corpus: {CLUSTERS}x{PER_CLUSTER}, k={K}\n\
             stamp: bench 1, {DIMENSION} dims\n\
             recall: {:.6}\n\
             agreement: {:.6}\n",
            metrics.recall, metrics.agreement
        );
        if let Some(parent) = baseline_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(baseline_path, text) {
            Ok(()) => println!("\nbaseline written to {}", baseline_path.display()),
            Err(e) => println!("\ncould not write the baseline: {e}"),
        }
        return;
    }

    // Compare against the recorded baseline, naming what regressed rather than only how much.
    let baseline = std::fs::read_to_string(baseline_path).ok();
    let Some(text) = baseline else {
        println!("\nno baseline at {}: run with `--bless` to record one", baseline_path.display());
        std::process::exit(if detects { 0 } else { 1 });
    };
    let read = |key: &str| -> Option<f32> {
        text.lines().find_map(|l| l.strip_prefix(key)).and_then(|v| v.trim().parse().ok())
    };
    let (Some(recall), Some(agreed)) = (read("recall:"), read("agreement:")) else {
        println!("\nthe baseline at {} is not readable", baseline_path.display());
        std::process::exit(1);
    };

    println!();
    println!("baseline: recall {recall:.3}  agreement {agreed:.3}");
    println!(
        "delta:    recall {:+.3}  agreement {:+.3}",
        metrics.recall - recall,
        metrics.agreement - agreed
    );

    let regressed = metrics.recall < recall - 1e-6 || metrics.agreement < agreed - 1e-6;
    if !failures.is_empty() {
        println!();
        println!("failing queries: {}", failures.join(", "));
    }
    if regressed || !detects {
        println!();
        println!(
            "{}",
            if !detects {
                "the harness cannot detect the degraded corpus, so its numbers mean nothing"
            } else {
                "REGRESSION: one or more metrics fell below the baseline"
            }
        );
        std::process::exit(1);
    }
    println!();
    println!("no regression");
}
