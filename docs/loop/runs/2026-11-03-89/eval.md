# Iteration 89 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **376** (373 + 3 new) and **46** lib (36 + 10 new) |
| 19 (per-frame) | **green** | 0.353 / 1.265 µs/frame at 64/256 |
| #22 crit 1 — centroid within 5 % | **green** | 220 / 1000 / 4000 / 8000 Hz, each within 5 % |
| #22 crit 2 — low-passed is darker | **green** | centroid **and** rolloff lower |
| #22 crit 3 — silence at the floor, not zero, not NaN | **green** | `SILENT_DB`, `centroid == None`, and `-0.0` too |
| #22 crit 4 — all three survive a round trip | **green** | in **both** formats, and silence stays distinct from a DC offset |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 376 + 46 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **20 GB** free; no disk interruption.

## Four defects, and what each taught

1. **A DC offset does not read `0 Hz`.** The test asserted it and failed with `Some(9.375001)`. A constant through a
   Hann window is a raised cosine, three bins wide, so the energy-weighted mean sits at **9.375 Hz**. The assertion now
   records the measured value and explains the leakage rather than hiding it.
2. **The "short clip" fixture was five windows long.** 0.1 s at 48 kHz is **4 800 samples** against a **1024-sample**
   window, so the short-clip branch was never entered and its assertion was vacuous. The fixture is now 10 ms.
3. **A three-tuple return failed `clippy::type_complexity`.** Replaced with an `Analysed` struct — which the call site
   reads better with anyway (`.spectral` rather than `.2`).
4. **A doc comment was orphaned** when the struct was inserted between it and its function, which `clippy` caught as
   `empty_line_after_doc_comments` — and then a stray blank line after the repair, caught the same way.

## The regression, stated as one

**Decode fell from 2 636 to ~1 150 clips/s** at 500 clips, measured three times (996, 1 145, 1 224). The new pass is a
second FFT sweep over the same frames, and the embedder's preprocessing is CLAP's, so nothing shares it.

This is **the case the iteration targeted** — a build now produces three more statistics per asset — so it is a cost,
not a violation of the 10 % rule. `perf.md` reports it, and 6 s for 5 000 clips remains far below the ~3 minutes the
audio tower itself would take.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–88.
