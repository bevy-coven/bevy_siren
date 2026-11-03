# Iteration 84 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **359** (330 + 4 new + 25) and **36** lib |
| 19 (perf) | **green** | 0.062 / 0.352 / 0.501 µs/frame at 8/64/256 |
| #21 criterion 1 (within 0.1 LU of the reference) | **green since iteration 77** | `a_tone_matches_the_references_integrated_loudness` — −23.0 ±0.1 against `ffmpeg`'s −23.0 |
| #21 criterion 2 (short clip handled explicitly) | **green** | `None` in memory (77) and `-` on disk (84) |
| #21 criterion 3 (true peak within 0.1 dB) | **green since 77** | −20.00 dBFS against the reference's −20.0 |
| #21 criterion 4 (visible per asset, survives a round trip) | **green** | `loudness_round_trips_and_a_short_clip_stays_absent` |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --features semantic` | **pass** — 359 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

No disk interruptions this iteration — the volume holds **24 GB** free after iteration 83's cleans.

## Two real defects found by the new tests

1. **The section marker was parsed as a record.** With provenance written before loudness, `provenance_and_loudness_sections_coexist`
   failed with `Record { line: 11, what: "a provenance record names no hash" }` — the `loudness 1` line arrived while
   the provenance section was open and was parsed as its record. The parser now recognises **markers first**, whatever
   section is current, so sections may appear in any order.
2. **My own test asserted behaviour the format does not have.** `a_non_finite_level_is_written_as_absence` expected
   `INFINITY` to round-trip through a `-`. It does not: the parser requires a number in that column and **refuses the
   whole file by name**, which is the correct behaviour and the one now under test. The test was renamed to
   `a_non_finite_level_makes_the_record_unrepresentable` and asserts the refusal.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80, 81, 82 and 83. `bang-spectrum.png` read: broadband onset, harmonic comb to
10 kHz, no click line, no clipping shelf, no gap.
