# Iteration 92 — eval

## The cases

`src/semantic/key.rs` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **376** and **80** lib (68 + 12 new) |
| 19 (per-frame) | **green** | 0.107 / 0.833 µs/frame at 64/256 |
| #25 crit 1 — accuracy on ≥20 known keys, exact and fifth separately | **green** | **24 keys, 100 % exact, 100 % fifth-compatible** |
| #25 crit 2 — an atonal asset has no key | **green** | a single impact, white noise, and silence |
| #25 crit 3 — the compatibility table is a named type | **green** | `Compatibility`, serde, with cited defaults |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 376 + 80 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **19 GB** free; no disk interruption.

## Six defects, and five of them were found by printing a number

1. **1024-point window cannot see a pitch.** 46.875 Hz/bin is **2.85 semitones at C4**; a pure C4 peaks in bin 6
   (**281 Hz**) and was read as C#. **4096** fixed the low octave.
2. **4096 cannot separate B4 from C5.** They are 29 Hz apart and a Hann main lobe is four bins; at 11.72 Hz/bin that is
   **2.5 bins**, so they merged and B tied the tonic. **8192** gives 5.0 bins.
3. **Band edges were half a semitone out of phase.** Note `n` spans `[n − 0.5, n + 0.5)`; the code compared `note` and
   `note + 1`, splitting C4's own partial between **B and C** (measured: B 10.71 against C 5.80 on a pure scale).
4. **Log-magnitude compression flattened the chroma** to 3.96–5.41 across all twelve classes — no contrast to correlate.
   Reverted to linear.
5. **The fixture was musically wrong.** A tonic pedal under the **V** chord is a **minor second** against its third; the
   passage genuinely held as much B as C, so "no key" was the correct answer. The bass now follows each chord's root.
6. **The confidence floor was below a single impact.** Measured 0.241 (progression) / 0.067 (impact) / 0.030 (noise);
   `0.05` let an impact report `D# minor`. It is **0.15**.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–91.
