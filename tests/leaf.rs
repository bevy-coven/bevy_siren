//! **The crate boundary, enforced.**
//!
//! `bevy_siren` is the Coven's sound crate: a cue table as data, buses that duck, a voice budget, layered music
//! and a log. It takes `bevy` (a trimmed feature set), and behind its features `serde`/`ron` for the tables as
//! data and `rodio` for its own mixer. It must never name a game, a game's camera, the editor that hosts it, or
//! a schema crate that would make the editor a dependency.
//!
//! The source scan forbids more than crate names: `MainCamera` is there because a crate extracted from a game is
//! tempted to name the game's camera, and the family has paid for that before. The editor (`bevy_grimoire`,
//! `bevy_cauldron`) is there because the editor is a **consumer** of this crate, never the other way round.

use std::path::{Path, PathBuf};

/// Everything this crate may depend on.
///
/// - `bevy` — the engine, trimmed to four features.
/// - `serde`, `ron` — the tables as data, and the file format a project edits.
/// - `rodio` — the crate's own mixer, behind `mixer`, **and the decoder the semantic feature uses**, behind
///   `semantic`. The version `bevy_audio` itself resolves, so the graph has one. The `semantic` entry costs no new
///   crate: `cargo tree -i rodio` reports `bevy_audio → bevy_internal → bevy → bevy_siren` already, so the entry
///   turns on this crate's decoder features and adds nothing to the graph.
/// - `rubato` — the resampler, behind `semantic`. It earns its place the way `ort` does, by measurement: `rodio`'s own
///   converter discards samples when down-sampling, and a hand-rolled interpolator produced a plausible embedding from
///   a corrupted signal (removed in iteration 62). `rubato`'s FFT resampler keeps a 1 kHz tone at **0.5000 of 0.5000**
///   of its energy with 15 kHz intact and a 1.000 s file at **exactly 48000** samples, measured in
///   `runs/2026-11-03-65/research.md`. Pure Rust: no C toolchain, no download.
/// - `ort` — the inference runtime, behind `semantic` and off by default. It earns its place by measurement rather
///   than by convenience: `docs/adr/0002-inference-runtime.md` chose it over `candle` because CLAP's graph is 39
///   standard operators and `ort` runs them exactly, and `runs/2026-11-03-63/evidence/ort-probe.txt` records it
///   loading the 501 MB text graph and returning a unit-norm embedding on this machine. A user who does not enable
///   `semantic` never compiles it.
/// - `ebur128` — loudness and true peak, behind `semantic`. `docs/adr/0003-acoustic-features.md` selected it after
///   checking the thing its name suggests and does not mean: it is **pure Rust**. Its `src/` is nine `.rs` files, its
///   C is confined to `capi-test/` and `tests/c/`, and its `build.rs` runs only under the `c-tests` feature, which is
///   off by default; with `default-features = false` a clean build takes **1.66 s**, invokes no `cc` and produces no C
///   object. It earns the entry by agreeing with `ffmpeg`'s reference to **0.01 LU** on every clip long enough to have
///   a 400 ms gating block, and its disagreement on shorter ones is why `semantic::loudness` carries a block count.
///   BS.1770's K-weighting and gating are not worth reimplementing to get a number a standard already fixes.
///
/// Adding a name here is a design decision, not a convenience: the sentence above is the argument, and it belongs in
/// the same commit as the entry.
const ALLOWED_DEPS: &[&str] = &["bevy", "serde", "ron", "rodio", "ort", "rubato", "ebur128"];

/// Crate and type names that would mean the boundary has been crossed: a physics engine, the editor, a particular
/// game, or a particular game's camera.
const FORBIDDEN_DEP_MARKERS: &[&str] =
    &["avian", "emerge", "sanctuary", "foundation_vs_slop", "MainCamera", "bevy_grimoire", "bevy_cauldron"];

fn crate_root() -> PathBuf {
    // Cargo runs a test binary with the cwd set to its own package root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rust_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Strip `//` line comments and `/* */` blocks so a doc comment *about* Bevy — of which this crate has several,
/// deliberately — is not mistaken for an import of it.
///
/// Deliberately simple: it does not track string literals, so a `"//"` inside a string would end the line early.
/// That is acceptable because the only thing this feeds is a substring search for crate names, and an over-eager
/// strip can only ever cause a false *pass* on a line that already contains no dependency.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_block = false;
    for line in src.lines() {
        let mut rest = line;
        loop {
            if in_block {
                match rest.find("*/") {
                    Some(end) => {
                        in_block = false;
                        rest = &rest[end + 2..];
                    }
                    None => break,
                }
            } else {
                let line_at = rest.find("//");
                let block_at = rest.find("/*");
                match (line_at, block_at) {
                    (Some(l), b) if b.is_none_or(|b| l < b) => {
                        out.push_str(&rest[..l]);
                        break;
                    }
                    (_, Some(b)) => {
                        out.push_str(&rest[..b]);
                        in_block = true;
                        rest = &rest[b + 2..];
                    }
                    _ => {
                        out.push_str(rest);
                        break;
                    }
                }
            }
        }
        out.push('\n');
    }
    out
}

/// No source file names a game, the editor or a physics engine.
#[test]
fn no_source_file_reaches_for_an_engine() {
    let src = crate_root().join("src");
    let mut files = Vec::new();
    rust_sources(&src, &mut files);
    assert!(
        files.len() >= 2,
        "expected to scan the whole crate, found only {} file(s) — has the layout moved?",
        files.len()
    );

    let mut offenders = Vec::new();
    for path in &files {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        let code = strip_comments(&text);
        for (n, line) in code.lines().enumerate() {
            for marker in FORBIDDEN_DEP_MARKERS {
                if line.contains(marker) {
                    offenders.push(format!(
                        "{}:{} — {}",
                        path.strip_prefix(crate_root()).unwrap_or(path).display(),
                        n + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "bevy_siren must not name a game or the editor, but {} line(s) do.\n  {}\n\n\
         This crate is sound any game can take. If a type genuinely needs to cross the boundary, the answer is a \
         plain-data type here and a conversion on the far side — not a dependency.",
        offenders.len(),
        offenders.join("\n  ")
    );
}

/// Only the four dependencies the crate argues for, and nothing that is a game or the editor.
#[test]
fn the_dependency_list_stays_closed() {
    let manifest =
        std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("bevy_siren must have a Cargo.toml");

    let deps =
        manifest.split("[dependencies]").nth(1).expect("bevy_siren must declare a [dependencies] table");
    let deps = deps.split("\n[").next().unwrap_or(deps);

    for line in deps.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let name = line.split(['=', ' ', '.']).next().unwrap_or("");
        // A dependency line is `name = ...` or `name.workspace = true`. Anything else inside this table is a
        // CONTINUATION of a multi-line value — a feature array's entries, a closing bracket.
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
            continue;
        }
        assert!(
            ALLOWED_DEPS.contains(&name),
            "bevy_siren declares `{name}`, which is not in its allowed set {ALLOWED_DEPS:?}.\n\
             Widening this is a design decision, not a convenience. If it is genuinely warranted, add it to \
             ALLOWED_DEPS in this test in the same commit, so the change is visible in review."
        );
        for marker in FORBIDDEN_DEP_MARKERS {
            assert!(
                !name.contains(marker),
                "bevy_siren declares `{name}` — the crate exists precisely so it does not depend on a game or the editor."
            );
        }
    }
}

/// The dependency set is exactly what the crate says it is: dropping one is as much a change as adding one.
#[test]
fn every_allowed_dependency_is_still_declared() {
    let manifest = std::fs::read_to_string(crate_root().join("Cargo.toml")).expect("a manifest");
    let deps = manifest.split("[dependencies]").nth(1).expect("a [dependencies] table");
    let deps = deps.split("\n[").next().unwrap_or(deps);
    for name in ALLOWED_DEPS {
        assert!(
            deps.lines().any(|l| l.trim_start().split(['=', ' ', '.']).next() == Some(*name)),
            "ALLOWED_DEPS names `{name}`, which the manifest no longer declares. A ratchet that permits more than \
             exists stops describing anything."
        );
    }
}
