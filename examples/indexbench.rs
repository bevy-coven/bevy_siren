//! **Indexing throughput**: how long a build takes, split into the part that decodes and the part that embeds.
//!
//! ```sh
//! cargo run --release --features semantic --example indexbench -- <library> [clips]
//! ```
//!
//! This is a different job from `perfcheck`, and the crate keeps them apart on purpose ([#36]'s own technical note): a
//! per-frame cost says nothing about a batch job, and one number must never be read as the other.
//!
//! # Why the split, and how it is measured
//!
//! An indexer is only as fast as its slowest stage, and the two stages scale differently: decoding is per byte and
//! scales with CPU cores, while inference is per clip and scales with the accelerator. **Reporting one number would
//! leave a reader unable to say which to fix**, so this measures:
//!
//! - **`decode`** — read, downmix and resample every file, with the embedder replaced by a no-op. This is the real
//!   `embed_measured` path with `embed` returning instantly, so it costs exactly what a build costs minus inference.
//! - **`embed`** — the same files through a synthetic embedder with fixed arithmetic cost, so the *shape* of the
//!   pipeline is measured without needing a checkpoint. Reported as "synthetic" because a real number needs the model.
//!
//! The ratio of the two is what identifies the bound, which is [#36]'s third acceptance criterion.
//!
//! [#36]: https://github.com/bevy-coven/bevy_siren/issues/36
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use bevy_siren::semantic::EMBEDDING_DIM;
use bevy_siren::semantic::index::ModelStamp;
use bevy_siren::semantic::indexer::build;

/// The cost a "synthetic" embedder imposes per clip, chosen so the two stages are both visible in the report rather
/// than one being lost in the timer's resolution. **Design default**, and named here so nobody reads the embed column
/// as a measurement of the model.
const SYNTHETIC_EMBED_WORK: usize = 512 * 512;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: cargo run --release --features semantic --example indexbench -- <library> [clips]");
        std::process::exit(2);
    }
    let root = PathBuf::from(&args[1]);
    let wanted: Option<usize> = args.get(2).and_then(|c| c.parse().ok());

    // **Build the corpus in a scratch directory**, so the bench does not need a licensed library and the number is
    // reproducible on any machine. A 0.5 s 1 kHz tone at 48 kHz mono: long enough to decode and to gate, short enough
    // that a thousand of them is a few megabytes.
    // **The corpus is generated when the path does not exist yet**, which is the ordinary way to run this: point it at
    // a fresh directory and it fills one. A path that exists and holds audio is used as given, so a real library can be
    // measured by passing it instead.
    let clips = wanted.unwrap_or(200);
    let needs_corpus =
        !root.is_dir() || std::fs::read_dir(&root).map(|mut d| d.next().is_none()).unwrap_or(true);
    if needs_corpus {
        std::fs::create_dir_all(&root)
            .map_err(|e| format!("cannot create `{}`: {e}", root.display()))
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(2);
            });
        for i in 0..clips {
            let path = root.join(format!("c{i:04}.wav"));
            if path.exists() {
                continue;
            }
            let mut file = std::fs::File::create(&path).expect("create");
            file.write_all(&tone(i)).expect("write");
        }
        println!("corpus: {} clips in {}", clips, root.display());
    }

    let stamp = ModelStamp::measured_clap();

    // **Decode only.** The embedder returns immediately, so this is the read-downmix-resample cost plus the loudness
    // pass, with inference removed.
    let start = Instant::now();
    let decoded = build(&root, stamp.clone(), |_| Ok(vec![0.0; EMBEDDING_DIM])).expect("build");
    let decode_s = start.elapsed().as_secs_f64();

    // **Decode plus a synthetic embed**, so the pipeline's second stage has a cost to separate.
    let start = Instant::now();
    let mut sink = 0.0f32;
    let embedded = build(&root, stamp, |buffer| {
        let mut acc = 0.0f32;
        for i in 0..SYNTHETIC_EMBED_WORK {
            acc += buffer[(i * 7) % buffer.len().max(1)] * 0.5;
        }
        sink += acc;
        Ok(vec![acc.min(1.0); EMBEDDING_DIM])
    })
    .expect("build");
    let embed_s = start.elapsed().as_secs_f64();

    let n = decoded.indexed().max(1) as f64;
    let per_decode = decode_s / n;
    let per_total = embed_s / n;
    let per_embed = (per_total - per_decode).max(0.0);

    println!();
    println!("{:>10}  {:>12}  {:>14}", "stage", "clips/s", "ms/clip");
    println!("{:>10}  {:>12.1}  {:>14.3}", "decode", 1.0 / per_decode, per_decode * 1000.0);
    println!(
        "{:>10}  {:>12.1}  {:>14.3}",
        "embed*",
        1.0 / per_embed.max(f64::MIN_POSITIVE),
        per_embed * 1000.0
    );
    println!("{:>10}  {:>12.1}  {:>14.3}", "total", 1.0 / per_total, per_total * 1000.0);
    println!();
    println!(
        "* synthetic: fixed {} multiply-adds per clip, not a model. Its ratio to `decode` is what names the bound.",
        SYNTHETIC_EMBED_WORK
    );
    println!(
        "  indexed {} / {} clips, {} unmeasured, {} failed, {} not audio",
        decoded.indexed(),
        n as usize,
        decoded.unmeasured,
        decoded.failed.len(),
        decoded.not_audio.len()
    );
    println!("  embedded run: {} indexed", embedded.indexed());
    for target in [500usize, 5_000] {
        let secs = per_total * target as f64;
        println!("  {} clips at this rate: {:.0} s ({:.1} min)", target, secs, secs / 60.0);
    }
    // A sink read so the optimiser cannot delete the synthetic work.
    if sink.is_nan() {
        println!("(sink {sink})");
    }
}

/// A 0.5 s 16-bit mono 48 kHz tone that is **different for every seed**, so the content hash is distinct for each.
///
/// The first version varied the frequency by `seed % 24`, which meant seeds 0, 24 and 48 produced **byte-identical
/// files** — and the scan, correctly, reported them as one asset with three paths. Measured: a 200-file corpus came back
/// as **24 assets** with zero skips and zero refusals, which is the scan working and the fixture lying. The frequency
/// now advances with the seed and a per-seed phase offset makes the bytes unique even where two seeds land near the
/// same Hz.
fn tone(seed: usize) -> Vec<u8> {
    let rate = 48_000u32;
    let frames = rate as usize / 2;
    let hz = 220.0 + (seed as f64) * 1.37;
    // A per-seed phase offset; `1/sqrt(2)` rather than a rounded literal, which clippy rightly flags.
    let phase = (seed as f64) * std::f64::consts::FRAC_1_SQRT_2;
    let mut v = Vec::with_capacity(44 + frames * 2);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + (frames * 2) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&rate.to_le_bytes());
    v.extend_from_slice(&(rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&((frames * 2) as u32).to_le_bytes());
    for i in 0..frames {
        let s = (std::f64::consts::TAU * hz * i as f64 / rate as f64 + phase).sin() * 0.5;
        v.extend_from_slice(&((s * f64::from(i16::MAX)) as i16).to_le_bytes());
    }
    v
}
