# Iteration 87 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **373** (367 + 6 new) and **36** lib |
| 19 (perf) | **green** | 0.062 / 0.354 / 1.312 µs/frame at 8/64/256 |
| the 3× claim, asserted | **green, new** | **10.28 MB** binary, **≈3.1×** text ratio on realistic values |
| both formats load the same index | **green, new** | vectors, provenance (space in a pack name) and loudness agree |
| truncation and foreign magic refused | **green, new** | checksum for a half file, `Bytes` for wrong magic and for 3 bytes |
| an old text index still loads | **green, new** | `read` sniffs the magic, so the file name does not decide |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --features semantic` | **pass** — 373 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

Volume held **21 GB** free; no disk interruption.

## Two defects the new tests caught, both in the tests

1. **The size fixture made the 3× claim look false.** The first version built 5 000 records of `vec![0.5f32; 512]` — a
   constant. `0.5` prints as **one character**, so the text form came out the same size as the binary form and the test
   failed with `the text form is ~3x the binary form: got 1.00`. The claim is about **real** embeddings; a fixture of
   repeated constants tests nothing about float formatting. Recomputed on a deterministic spread: **6 376.4 bytes** per
   text record, **31.88 MB** for 5 000 — ADR 0004's number to the byte.
2. **A refusal message asserted the wrong branch.** `b"\x00\x01 not an index"` is **valid UTF-8**, so it reaches
   `Index::load` and gets a precise *"unreadable index header"* rather than the vaguer *"neither a binary index nor
   UTF-8"*. Both paths are now tested with inputs that actually take them — the specific refusal is the better one, and
   the test says so.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–86. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no
click line, no clipping shelf, no gap.
