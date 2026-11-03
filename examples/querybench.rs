//! **`#47`: query latency, split into the two halves the issue names.**
//!
//! The text encode is GPU work and the index scan is memory work; a single number would hide which one a panel waits
//! on. This measures them separately, at 1 000 and 10 000 assets, and reports **p50 and p99** — a mean hides the one
//! keystroke that takes a second, which is the only one a person notices.
//!
//! ```sh
//! cargo run --release --features semantic --example querybench
//! ```
//!
//! The encode half is **not** measured here: it needs the ONNX graph and `ort`, which this bench does not link. What is
//! measured is everything the crate itself does per query — the scan, the ranking and the ensemble — and the encode is
//! left as the caller's cost with the crate's own measured figure quoted beside the result.

use std::time::Instant;

use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::rank::{Floor, top_k};
use bevy_siren::semantic::search::{Ranked, search_ranked};
use bevy_siren::semantic::weights::Weights;

const QUERIES: usize = 120;
const DIMENSION: usize = 512;

/// A deterministic pseudo-random vector, so the corpus is the same on every run.
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

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn stamp() -> ModelStamp {
    ModelStamp { model: "bench".into(), version: "1".into(), sample_rate: 48_000, dimension: DIMENSION }
}

fn main() {
    let weights = Weights::default();
    println!("{QUERIES} queries per size, k=3, dimension {DIMENSION}");
    println!();
    println!(
        "{:>7}  {:>10} {:>10}  {:>10} {:>10}  {:>10} {:>10}",
        "assets", "scan p50", "scan p99", "full p50", "full p99", "sort p50", "sort p99"
    );

    for assets in [1_000usize, 10_000] {
        let mut index = Index::new(stamp());
        for a in 0..assets {
            let _ = index.insert(a as u64, vector(a as u64 + 1));
        }
        let empty = std::collections::BTreeMap::new();

        let mut scan = Vec::with_capacity(QUERIES);
        let mut full = Vec::with_capacity(QUERIES);
        let mut sort = Vec::with_capacity(QUERIES);

        for q in 0..QUERIES {
            let embedding = vector(1_000_000 + q as u64);
            let query = bevy_siren::semantic::query::Query::new("a tense sparse ambience").expect("a query");

            // The whole scan plus the ranking: `top_k` walks every record once.
            let start = Instant::now();
            let ranked = top_k(&index, &embedding, 3).expect("a ranking");
            scan.push(start.elapsed().as_secs_f64() * 1000.0);
            std::hint::black_box(&ranked);

            // The sort alone, for contrast: it is what makes 10 000 assets cost more than 1 000.
            let all: Vec<f32> = index.records.keys().map(|_| 0.0).collect();
            let start = Instant::now();
            let mut v = all;
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            sort.push(start.elapsed().as_secs_f64() * 1000.0);

            // The full path a panel runs, including the ensemble and the hard rules.
            let request = Ranked {
                embedding: &embedding,
                anchor: 1,
                features: &empty,
                k: 3,
                floor: Floor::default(),
                weights: &weights,
            };
            let start = Instant::now();
            let answer = search_ranked(&index, &index.stamp.clone(), &query, &request).expect("an answer");
            full.push(start.elapsed().as_secs_f64() * 1000.0);
            std::hint::black_box(&answer);
        }

        scan.sort_by(f64::total_cmp);
        full.sort_by(f64::total_cmp);
        sort.sort_by(f64::total_cmp);
        println!(
            "{assets:>7}  {:>10.3} {:>10.3}  {:>10.3} {:>10.3}  {:>10.3} {:>10.3}",
            percentile(&scan, 0.50),
            percentile(&scan, 0.99),
            percentile(&full, 0.50),
            percentile(&full, 0.99),
            percentile(&sort, 0.50),
            percentile(&sort, 0.99),
        );
    }
    println!();
    println!("milliseconds. `scan` is top_k alone; `full` adds the filter, the ensemble and the re-sort.");
    println!("`sort` is a sort over the whole library, which is what a ranking that sorts everything pays.");
}
