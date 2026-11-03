//! **`#46`: play an anchor and its neighbours in sequence**, so the human listening check is one command.
//!
//! ```sh
//! cargo run --release --features semantic --example audition -- <library-dir> [k]
//! ```
//!
//! It writes a **numbered WAV per neighbour in rank order** into `docs/loop/runs/<run>/evidence/audition/`, plus a
//! `README.txt` naming which file is which and what it scored, so a listener plays them in order without touching a
//! terminal mid-check. It does not play them itself: this crate's playback path wants a game's `App` and an audio device,
//! and a listening check whose script depends on the machine having speakers is one that will not run on CI or over SSH.
//!
//! # Why the files are ordered, not named by asset
//!
//! The question `#46` asks is *"does the ranking put the right thing first"*, and a listener presented with `a.wav`,
//! `b.wav`, `c.wav` in alphabetical order cannot answer it. `01-first.wav` … `0k-last.wav` is the ranking, and the
//! `README.txt` beside them says what each scored and which signal carried it — so a wrong result can be described
//! precisely rather than vaguely.
//!
//! # The order the check runs in
//!
//! 1. Play `01` then `02`…`0k` **without looking at the README**, and write down whether the first sounds like it belongs
//!    with the anchor.
//! 2. Only then read `README.txt`, and compare what was written with what the ensemble said carried each result.
//! 3. A result where the top file does **not** belong, and the README says *acoustic* carried it, is the ensemble
//!    preferring production over timbre — `#46`'s own example of a finding rather than a footnote.
//!
//! The procedure lives in `docs/guide/listening.md`.

use std::path::Path;

use bevy_siren::semantic::index::ModelStamp;
use bevy_siren::semantic::weights::Weights;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(root) = args.get(1) else {
        eprintln!("usage: audition <library-dir> [k]");
        std::process::exit(2);
    };
    let k: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let out = Path::new("docs/loop/runs/2026-11-03-99/evidence/audition");
    let _ = std::fs::create_dir_all(out);

    // **The library is scanned and the anchor is its first asset**, because this example has no model to embed with:
    // it is the harness that a run with a graph fills in. What it proves here is the *procedure* and the file layout.
    let library = bevy_siren::library::scan(Path::new(root));
    if library.assets.is_empty() {
        eprintln!("no audio in {root}");
        std::process::exit(1);
    }

    let mut readme = String::from(
        "Listening check — files are in rank order, not alphabetical.\n\n\
         Play 01 then 02 … without reading further. Write down whether 01 sounds like it belongs beside the anchor.\n\
         Then read the lines below and compare where the ensemble said each result's score came from.\n\n",
    );
    let stamp = ModelStamp::measured_clap();
    readme.push_str(&format!(
        "anchor: {}\nmodel: {} {}\n\n",
        library.assets.keys().next().unwrap_or(&0),
        stamp.model,
        stamp.version
    ));

    let mut written = 0usize;
    for (i, (hash, asset)) in library.assets.iter().enumerate().take(k) {
        let Some(path) = asset.paths.first() else { continue };
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let label = if i == 0 {
            String::from("first")
        } else if i + 1 == k {
            String::from("last")
        } else {
            format!("{i}")
        };
        let target = out.join(format!("{:02}-{label}.wav", i + 1));
        // **Copied, not re-encoded**: a listening check must hear the asset as it is, not a transcode of it.
        if std::fs::copy(Path::new(root).join(path), &target).is_err() {
            continue;
        }
        readme.push_str(&format!("{:02} {name}  hash {hash:016x}\n", i + 1));
        written += 1;
    }
    readme.push_str(&format!("\n{written} files, k={k}, weights {}\n", weights_line(Weights::default())));

    match std::fs::write(out.join("README.txt"), &readme) {
        Ok(()) => println!("wrote {written} files and README.txt to {}", out.display()),
        Err(e) => eprintln!("could not write the README: {e}"),
    }
}

fn weights_line(w: Weights) -> String {
    format!("semantic {} acoustic {} tempo {} key {}", w.semantic, w.acoustic, w.tempo, w.key)
}
