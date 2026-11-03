# Iteration 86 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **367** (362 + 5 new) and **36** lib |
| 19 (perf) | **green** | 0.059 / 0.329 / 1.141 µs/frame at 8/64/256 |
| a sidecar reaches the index | **green, new** | one record on the right hash; the unnamed file absent |
| a typo is counted | **green, new** | `unattributed == 1`, named in the summary |
| an unreadable sidecar is reported | **green, new** | audio indexed, provenance empty, reason carried |
| no sidecar is normal | **green, new** | `"1 indexed"` and nothing extra |
| a pack name with a space survives | **green, new** | through the build **and** `to_text`/`load` |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --features semantic` | **pass** — 367 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

Volume held **21 GB** free; no disk interruption.

## What the new fields forced, and why that is the right sign

`Built` is a public struct with public fields, so adding `unattributed` and `sidecar` broke two test literals with
`missing fields ... in initializer of Built`. That is the compiler confirming the change is **observable** — a consumer
that constructs `Built` (which only this crate's own tests do) must acknowledge the new facts rather than inherit a
default that hides them.

The alternative — `#[non_exhaustive]` or a builder — would have made the change invisible at the construction site, which
for a struct whose job is *reporting what a run did* is the wrong trade.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–85. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no
click line, no clipping shelf, no gap.
