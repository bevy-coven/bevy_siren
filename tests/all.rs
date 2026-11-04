//! Every integration test in one binary.
//!
//! Each file in `tests/` would otherwise be its own test executable, and each links its own copy of Bevy: 650 to
//! 790 MB apiece in the family's measured case, 24 GB for one relink. They are modules of this one now
//! (`autotests = false` in `Cargo.toml`), so a test is `all::<file>::<name>`. A new file in `tests/` is declared
//! here, or `every_test_file_is_a_module` fails: an undeclared file is never compiled and its tests never run.

mod ensemble;
mod failed_load;
mod helpers;
mod leaf;
mod library;
mod mix;
#[cfg(feature = "mixer")]
mod mixer;
mod model;
mod music;
mod neutralised;
mod ogg_loops;
mod one_answer;
mod owner;
mod play;
mod quick_start;
mod readme;
mod render_shapes;
#[cfg(feature = "semantic")]
mod semantic_ask;
#[cfg(feature = "semantic")]
mod semantic_budget;
#[cfg(feature = "semantic")]
mod semantic_buffer;
#[cfg(feature = "semantic")]
mod semantic_decode;
#[cfg(feature = "semantic")]
mod semantic_ensemble;
mod semantic_eval;
#[cfg(feature = "semantic")]
mod semantic_golden;
#[cfg(feature = "semantic")]
mod semantic_incremental;
#[cfg(feature = "semantic")]
mod semantic_index;
#[cfg(feature = "semantic")]
mod semantic_indexer;
#[cfg(feature = "semantic")]
mod semantic_model;
#[cfg(feature = "semantic")]
mod semantic_query;
#[cfg(feature = "semantic")]
mod semantic_rank;
#[cfg(feature = "semantic")]
mod semantic_resample;
#[cfg(feature = "semantic")]
mod semantic_search;
#[cfg(feature = "semantic")]
mod semantic_server;
#[cfg(feature = "semantic")]
mod semantic_session;
#[cfg(feature = "semantic")]
mod semantic_tokenizer;
#[cfg(feature = "semantic")]
mod semantic_tools;
mod semantic_wait;
mod semantic_weights;
#[cfg(feature = "serde")]
mod table;
mod undecodable;

/// The files in `tests/` and the `mod` lines above are the same set.
#[test]
fn every_test_file_is_a_module() {
    const THIS_FILE: &str = include_str!("all.rs");
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
    let entries = std::fs::read_dir(dir).expect("tests/ is readable");
    let mut missing = Vec::new();
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("a UTF-8 file name");
        if stem == "all" {
            continue;
        }
        if !THIS_FILE.lines().any(|l| l == format!("mod {stem};")) {
            missing.push(stem.to_owned());
        }
    }
    assert!(
        missing.is_empty(),
        "tests/{{{}}}.rs is not declared as a `mod` in tests/all.rs, so it never compiles or runs",
        missing.join(",")
    );
}

/// Every file under `src/` outside `model.rs` is free to name `bevy`; **`model.rs` is not.** It is the engine-free
/// half, and a `Vec3` or a `Time` in it would make the crate's arithmetic untestable without an app.
mod engine_free {
    #[test]
    fn model_names_no_engine_type() {
        let src = include_str!("../src/model.rs");
        let body: String =
            src.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        for forbidden in ["bevy::", "Vec3", "Quat", "Entity", "Commands", "Res<", "Query<"] {
            assert!(
                !body.contains(forbidden),
                "src/model.rs names `{forbidden}`, which makes the crate's arithmetic need an app to test"
            );
        }
    }

    /// **The ensemble arithmetic is in the scanned file, not beside it.**
    ///
    /// `#40`'s own note says to put the scoring where `src/model.rs` keeps its arithmetic. That is a rule, not a
    /// preference: `model_names_no_engine_type` scans exactly one file, so a score defined in `semantic/` would be
    /// outside the scan and could quietly grow a `Res<…>` parameter — and then nothing tests it without an `App`.
    ///
    /// This asserts the types are *there*, so moving them out is a failing test rather than a silent exemption.
    #[test]
    fn the_ensemble_lives_in_the_engine_free_model() {
        let src = include_str!("../src/model.rs");
        for item in ["pub struct Breakdown", "pub struct Contribution", "pub enum Signal", "pub fn score"] {
            assert!(
                src.contains(item),
                "src/model.rs must define `{item}`, so the engine-free scan covers it"
            );
        }
    }

    /// A cue table, a mix table and a score read from RON survive a round trip with every field a project omits
    /// filled from its default.
    #[cfg(feature = "serde")]
    #[test]
    fn the_tables_round_trip_through_ron() {
        use bevy_siren::{CueTable, MusicScore, SirenSettings};

        let cue_text =
            "(\n    cues: [\n        (name: \"footstep.grass\", sounds: [(sound: \"step_01\")]),\n    ],\n)";
        let table = CueTable::from_ron(cue_text).expect("the smallest cue table parses");
        assert_eq!(table.cues[0].bus_name(), "Effects");
        assert_eq!(table.cues[0].max_instances, 1);
        assert_eq!(table.to_ron().map(|t| CueTable::from_ron(&t)), Ok(Ok(table.clone())));

        let settings =
            SirenSettings::from_ron("(master_db: -3.0)").expect("a settings file with one field parses");
        assert_eq!(settings.master_db, -3.0);
        assert_eq!(settings.max_voices, 32);
        assert_eq!(settings.buses.len(), 4);
        assert_eq!(settings.ducks.len(), 2);

        let score = MusicScore::from_ron("(layers: [], tempo_bpm: 120.0)").expect("a score parses");
        assert_eq!(score.beats_per_bar, 4.0);
        assert!(score.problems(&CueTable::default()).iter().any(|p| p.contains("no layers")));
    }
}
