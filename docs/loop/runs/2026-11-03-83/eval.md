# Iteration 83 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature; no audio-path file changed.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **355** (was 344 + 5 new + 6) and **36** lib |
| 19 (perf) | **green** | 0.065 / 0.358 / 1.038 µs/frame at 8/64/256 |
| #14 criterion 1 | **green, now through the file** | `an_empty_licence_survives_the_file_distinguishably_from_an_absent_one` |
| #14 criterion 2 | **half** — the reader and the store are both built; linking filename → hash is the remaining step | `Sidecar::read`, `Index.provenance` |
| #14 criterion 3 | **green** | `a_search_result_carries_its_provenance` |
| #14 criterion 4 | **green** | `a_result_cannot_be_built_without_provenance` + the type’s `Option<bool>` |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --features semantic` | **pass** — 355 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

**Two disk interruptions**, both handled with the documented fix: `cargo clean -p bevy_siren` freed **5.1 GB** and then
**6.1 GB**. The volume then read **27 GB free** — far more than either clean returned — so something else on the machine
also released space mid-run.

## Real defects found and fixed during the iteration

1. **A test fixture that did not test what it claimed.** `a_search_result_carries_its_provenance` inserted hashes `1`
   and `2` on top of `index_with(2)`'s existing `0x1000`-based hashes, so four vectors ranked and the assertion read
   `left: 4097, right: 2`. The test was wrong, not the code; it now builds a fresh index so its two hashes are the only
   ones.
2. **`clippy::bool_comparison`**: `is_recorded() == false` → `!is_recorded()`. A small thing, and exactly what a
   `-D warnings` gate is for.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80, 81 and 82. `bang-spectrum.png` read: broadband onset, harmonic comb to
10 kHz, no click line, no clipping shelf, no gap.
