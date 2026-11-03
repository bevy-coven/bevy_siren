# Iteration 81 — eval

## The cases

No audio-path file changed (`src/model.rs`, `cue.rs`, `mix.rs`, `play.rs`, `music.rs` … untouched); the work is
`src/semantic/` behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198 passed**; `--features serde` **207**; `--features mixer` **208** |
| 19 (perf) | **green** | 0.064 / 0.355 / 1.341 µs/frame at 8/64/256 |
| #16 criterion 1 | **green** | `adding_one_file_embeds_exactly_one` |
| #16 criterion 2 | **green** | `a_deleted_file_leaves_no_vector` |
| #16 criterion 3 | **green** | `a_rename_costs_nothing` |
| #16 criterion 4 | **green** | `every_run_reports_the_three_counts` |
| the re-encode decision | **green** | same test: a rewritten file is an add **and** a removal, never "unchanged" |

## The defect this iteration found in its own draft

`an_undecodable_file_is_failed_not_counted` failed on the first run: a corrupt `broken.wav` appeared in **no count at
all** — not added, not unchanged, not failed. `update` was iterating `library.assets` and ignoring `library.skipped`,
so the scan's refusal of a file that *is* audio by extension vanished. `indexer::build` splits those skips by meaning and
reports the decodable-but-broken ones; the incremental path did not.

**That is precisely the silence this crate refuses everywhere else**, reproduced in the new code. The test caught it
because it asserts a *count* rather than that "something sensible happened".

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --lib --features semantic` | **pass** — 25 |
| `cargo test --features semantic --test all semantic_incremental` | **pass** — 6 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

**Two disk interruptions, both handled with the documented fix.** `cargo clean -p bevy_siren --target-dir target`
freed **4.3 GB** and **4.3 GB** on two separate occasions, each time taking the volume from ~240 MB back to usable.
The `--features mixer` run reported two doctest **link** failures at 229 MB free; re-running after the clean gave
**208 passed, 0 failed**, confirming they were the disk and not the code.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75 and 80 — the audio path is untouched.
