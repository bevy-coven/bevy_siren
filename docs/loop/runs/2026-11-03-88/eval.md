# Iteration 88 — eval

## The cases

`examples/indexbench.rs` plus one `Cargo.toml` entry. **No `src/` change**, so no compiled behaviour moved.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6**; semantic **373** |
| 19 (per-frame) | **green** | 0.047 / 0.284 / 1.051 µs/frame at 8/64/256 |
| #36 clips per second | **green, new** | 2 411 / 2 636 / 2 724 decode-only clips/s at 200 / 500 / 1000 |
| #36 5 000-clip projection | **green, new** | **3 s** at the measured rate |
| #36 bound identified | **partly** | decode is 0.37–0.42 ms/clip in the bench; a real CPU embed measured 34.9 ms/clip in ADR 0002, so **inference dominates on CPU** — stated rather than claimed from a synthetic |
| #36 peak VRAM on the RTX 3090 | **open** | needs a current checkout and the graph on `big`; see below |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 373 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings, **including the new example** |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **21 GB** free; no disk interruption.

## Three defects, all mine, all found by running the thing

1. **The bench refused to create its own corpus.** The first version exited `"/tmp/… is not a directory"` before the
   `create_dir_all` that would have made it one. Caught by the first invocation.
2. **The fixture collapsed 200 files into 24 assets, and the bench printed success.** `tone()` varied frequency by
   `seed % 24`, so seeds 0, 24, 48 produced **byte-identical** WAVs (`shasum` confirms three files share one hash). The
   scan correctly returned `assets 24, skipped 0, refused 0` — one content at several paths, which is what
   `Library::duplicated` exists for. **The bench hid it** by printing `indexed 24 / 24`, a ratio whose denominator was
   derived from its numerator.
3. **A magic constant in the phase offset.** `0.7071` is `1/√2`; `clippy::approx_constant` caught it and the constant is
   now named.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–87.
