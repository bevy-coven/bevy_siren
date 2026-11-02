//! Frame cost of the mix at 8, 64 and 256 voices, so an iteration's `perf.md` is a measurement.
//!
//! ```sh
//! cargo run --release --example perfcheck
//! ```
//!
//! It costs the crate's own per-frame work, not the audio thread's: `model::chain_db` and `model::db_to_gain` for every
//! live voice, which is what `update_mix` does per voice whose gain moved, and `model::approach` per bus. A voice whose
//! gain did not move is skipped by the real system, so this is an **upper bound** and says so.
use std::time::Instant;

use bevy_siren::model::{MIX_TAU_S, approach, chain_db, db_to_gain};
use bevy_siren::{BusLevel, SirenSettings};

fn main() {
    let settings = SirenSettings {
        max_voices: 256,
        buses: vec![
            BusLevel::new("Music").gain_db(-2.0),
            BusLevel::new("Effects"),
            BusLevel::new("Ambience").gain_db(-6.0),
            BusLevel::new("Dialogue"),
        ],
        ..SirenSettings::default()
    };
    let mut sink = 0.0f32;
    println!("{:>6}  {:>12}  {:>14}", "voices", "us/frame", "us/voice");
    for voices in [8usize, 64, 256] {
        let gains: Vec<f32> = (0..voices).map(|i| -0.05 * (i % 40) as f32).collect();
        // Frames are cheap; run enough of them for the clock to resolve the mean.
        let frames = 2_000;
        let start = Instant::now();
        for _ in 0..frames {
            let mut bus = 0.0f32;
            for bus_level in &settings.buses {
                bus = approach(bus, bus_level.gain_db, MIX_TAU_S, 1.0 / 60.0);
            }
            for gain in &gains {
                sink += db_to_gain(chain_db(&[*gain, bus]));
            }
        }
        let elapsed = start.elapsed().as_secs_f64();
        let per_frame = elapsed / f64::from(frames) * 1e6;
        println!("{voices:>6}  {per_frame:>12.3}  {:>14.4}", per_frame / voices as f64);
    }
    // A black box the optimiser cannot delete the loop out of.
    std::hint::black_box(sink);
    println!();
    println!(
        "upper bound: every voice is costed, including the ones update_mix skips because their gain did not move"
    );
}
