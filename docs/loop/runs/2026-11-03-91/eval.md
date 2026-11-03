# Iteration 91 — eval

## The cases

`src/semantic/tempo.rs` only, behind the non-default `semantic` feature. No `src/` behaviour changed outside it.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **376** and **68** lib (57 + 11 new) |
| 19 (per-frame) | **green** | 0.282 / 1.055 µs/frame at 64/256 |
| #24 crit 1 — accuracy within ±2 BPM on ≥20 tracks | **green** | **80 tracks**, every whole BPM 80–159, all inside the floor |
| #24 crit 2 — octave errors counted separately | **green** | `is_octave_of` against `within_floor`, and never conflated |
| #24 crit 3 — a non-rhythmic asset reports no tempo | **green** | a single impact **and** a steady tone, both `None` |
| #24 crit 4 — the floor documented, absent treated as unknown | **green** | `FLOOR_BPM`'s doc, and `bpm: Option` |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 376 + 68 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **20 GB** free; no disk interruption.

## Four defects, and the first is the interesting one

1. **A steady 440 Hz tone was reported as `150.1 BPM` at confidence 0.29.** A false positive of exactly the kind `#24`
   warns about, and it **survived the first fix** because the constant's **unit was wrong**: the gate compared an FFT
   energy against a mean sample energy and used `0.02`, justified by a windowed time-domain energy measured by hand.
   The real ratios are **0.28 (tone)** and **4.1 × 10⁵ (click track)**, so `0.02` sat *below* the tone and let it
   through. The gate is now **2.0**, with the measured values in its doc so a reader can check the unit.
2. **A doc link to `[tests]`**, which is not an item; `rustdoc -D warnings` refused it. The same sentence also said
   *"twenty"* tempi where the corpus has **eighty** — the doc was stale in two ways at once.
3. **`clippy::needless_range_loop`** on the impact-burst fixture; rewritten as an iterator.
4. **A `mean_onset` computation computed twice** — once for the gate, once for the mean-removal below. Left as-is
   deliberately: the second is the mean of the *same* slice and folding them would save one pass over ~400 floats while
   making the gate read as though it shared state with the correlation. Not worth the obscurity.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–90.
