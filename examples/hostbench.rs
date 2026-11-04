//! **`#28`'s pitfall, answered with numbers**: what `siren-tools` costs to start, and what a call costs once it is running.
//!
//! > *"A tool server that holds the index in memory will re-load it per process start. State the load cost and whether a
//! > long-lived server is the expectation."*
//!
//! ```sh
//! cargo run --release --features semantic --example hostbench
//! ```
//!
//! Two figures, kept apart for the reason `querybench` keeps its halves apart — **a start-up cost and a per-call cost must
//! never be read as the same number**:
//!
//! - **start-up**: writing the index, reading it back through `indexer::read` (the same call the binary makes), and building
//!   the host (which derives every asset's acoustic features once). It is paid **once per process**, which is why a
//!   long-lived server is the expectation: the stdio transport launches one per client session.
//! - **per call**: the whole of `Host::handle` — parse the line, validate, run the tool, serialise the reply — at p50 and
//!   p99, because a mean hides the one call a person waits on.
//!
//! What is **not** measured: loading the ONNX graph, which needs the 501 MB model this bench does not link, and the text
//! encode that `search_music` pays. `search_music` here uses a fixed embedding, so its figure is the scan, the ranking and
//! the serialisation — the crate's own part — exactly as `querybench` states for the same reason.

use std::time::Instant;

use bevy_siren::semantic::index::{Index, ModelStamp};
use bevy_siren::semantic::indexer;
use bevy_siren::semantic::key::{Estimate, Key};
use bevy_siren::semantic::loudness::Loudness;
use bevy_siren::semantic::query::Query;
use bevy_siren::semantic::server::{Embed, Host};
use bevy_siren::semantic::spectral::Spectral;
use bevy_siren::semantic::tempo::Tempo;

const CALLS: usize = 60;
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

/// An embedder that answers one vector, so the figure is the crate's work and not a model's.
struct Fixed(Vec<f32>);

impl Embed for Fixed {
    fn embed(&mut self, _query: &Query) -> Result<Vec<f32>, String> {
        Ok(self.0.clone())
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

fn id(n: u64) -> String {
    format!("{n:016x}")
}

/// A fully analysed library: every asset has a loudness, a spectrum, a tempo and a key, so a compatibility query pays for
/// every lookup a real one does.
fn library(assets: usize) -> Index {
    let mut index = Index::new(ModelStamp::measured_clap());
    for a in 1..=assets as u64 {
        let _ = index.insert(a, vector(a));
        let f = a as f32;
        index.loudness.insert(
            a,
            Loudness {
                integrated: Some(-30.0 + (f % 12.0)),
                true_peak: -1.0,
                sample_peak: -1.5,
                blocks: 20,
                blocks_possible: 20,
            },
        );
        index.spectral.insert(
            a,
            Spectral {
                centroid_hz: Some(500.0 + f % 4000.0),
                rolloff_hz: Some(2000.0 + f % 8000.0),
                noise_floor_db: -70.0 + (f % 20.0),
            },
        );
        index.tempo.insert(a, Tempo { bpm: Some(70.0 + f % 90.0), confidence: 0.7, octave_ambiguous: false });
        index.key.insert(
            a,
            Estimate { key: Key { tonic: Some((a % 12) as u8), major: a % 2 == 0 }, confidence: 0.5 },
        );
    }
    index
}

fn main() {
    println!("{CALLS} calls per tool, dimension {DIMENSION}, k=3 (sfx k=6)");
    println!();
    println!("start-up, paid once per process");
    println!("{:>7}  {:>9}  {:>10}  {:>12}", "assets", "file MB", "read ms", "Host::new ms");

    let mut rows = Vec::new();
    for assets in [1_000usize, 10_000] {
        let index = library(assets);
        let dir = std::env::temp_dir().join(format!("bevy_siren_hostbench_{}_{assets}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        let path = indexer::write(&dir, &index).expect("the index is written");
        let megabytes = std::fs::metadata(&path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0);

        let start = Instant::now();
        let loaded = indexer::read(&dir, &ModelStamp::measured_clap()).expect("the index reads back");
        let read_ms = start.elapsed().as_secs_f64() * 1000.0;

        let start = Instant::now();
        let mut host =
            Host::new(loaded, ModelStamp::measured_clap(), Some(Box::new(Fixed(vector(1_000_003)))));
        let new_ms = start.elapsed().as_secs_f64() * 1000.0;
        let _ = std::fs::remove_dir_all(&dir);
        println!("{assets:>7}  {megabytes:>9.2}  {read_ms:>10.1}  {new_ms:>12.1}");

        let line = |tool: &str, arguments: serde_json::Value| {
            serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": arguments } })
                .to_string()
        };
        // The anchor first, so the two compatibility tools have one.
        let _ = host.handle(&line("set_anchor", serde_json::json!({ "asset_id": id(1) })));
        let calls = [
            ("set_anchor", line("set_anchor", serde_json::json!({ "asset_id": id(1) }))),
            ("find_compatible_music", line("find_compatible_music", serde_json::json!({ "k": 3 }))),
            ("find_compatible_sfx", line("find_compatible_sfx", serde_json::json!({ "k": 6 }))),
            (
                "explain_match",
                line("explain_match", serde_json::json!({ "asset_id": id(2), "anchor": id(1) })),
            ),
            (
                "search_music",
                line("search_music", serde_json::json!({ "description": "a tense sparse ambience", "k": 3 })),
            ),
        ];
        for (tool, request) in calls {
            let mut times = Vec::with_capacity(CALLS);
            let mut reply_bytes = 0;
            for _ in 0..CALLS {
                let start = Instant::now();
                let reply = host.handle(&request).expect("a call is answered");
                times.push(start.elapsed().as_secs_f64() * 1000.0);
                reply_bytes = reply.len();
                std::hint::black_box(&reply);
            }
            times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            rows.push((assets, tool, percentile(&times, 0.5), percentile(&times, 0.99), reply_bytes));
        }
    }

    println!();
    println!("per call, the whole of `Host::handle` (parse, validate, run, serialise)");
    println!("{:>7}  {:<22} {:>9} {:>9}  {:>11}", "assets", "tool", "p50 ms", "p99 ms", "reply bytes");
    for (assets, tool, p50, p99, bytes) in rows {
        println!("{assets:>7}  {tool:<22} {p50:>9.3} {p99:>9.3}  {bytes:>11}");
    }
}
