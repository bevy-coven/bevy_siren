# Iteration 82 — eval

## The cases

`src/semantic/provenance.rs` is behind the non-default `semantic` feature; no audio-path file changed.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| 19 (perf) | **green** | 0.062 / 0.342 / 1.227 µs/frame at 8/64/256 |
| #14 criterion 1 (explicit unknown) | **green** | `an_absent_licence_is_distinguishable_from_an_empty_one` |
| #14 criterion 4 (never permissive) | **green** | `unknown_provenance_never_reads_as_permissive` |
| #14 criterion 2 (sidecar reaches the index) | **builder done, storage decided elsewhere** | `Sidecar` reads CSV and JSON; `needs-decision/provenance-in-the-index.md` |
| #14 criterion 3 (a result reports it) | **decided elsewhere** | `rank::Match` losing `Copy` is a public-type change |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --lib --features semantic` | **pass** — 36 (25 before, 11 new) |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

**One disk interruption**, handled with the documented fix: `cargo clean -p bevy_siren` freed **4.9 GB** after a
`--features serde` link failure at 1.4 GB free, which then ran **207 passed**.

## The defect this iteration found in its own draft

The permissive-licence check was written as `lower.contains("mit")` — a **substring** match. That accepts
`"resubmitted-terms"` as the MIT licence, which is the failure direction that *grants* permission, in a function whose
entire job is to refuse to over-claim. It passed all tests as written because no test used a string containing `mit` by
accident.

Fixed to a **leading stem** match, with `resubmitted-terms → None` now pinned as a test, alongside `CC-BY-4.0`,
`CC-BY-SA-4.0` and `CC-BY-ND-4.0` all correctly reading commercial (share-alike and no-derivatives are separate
obligations, not commercial restrictions).

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80 and 81. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz,
no click line, no clipping shelf, no gap.
