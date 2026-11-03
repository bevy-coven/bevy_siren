# Iteration 85 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **362** (359 + 3 new) and **36** lib |
| 19 (perf) | **green** | 0.063 / 0.365 / 0.963 µs/frame at 8/64/256 |
| loudness is produced by a build | **green, new** | a 3 s tone stores ≈ **−9.0 LUFS**, 27 blocks, `unmeasured == 0` |
| a short file stored unmeasured | **green, new** | 0.2 s → `integrated: None`, finite peak, `too_short()` && !`silent()` |
| an unmeasured run says so | **green, new** | `describe()` → `"0 indexed, 3 unmeasured"` |

## Baseline

| Step | Result |
|---|---|
| `cargo build` | **pass** |
| `cargo test` | **pass** — 198 |
| `cargo test --features serde` | **pass** — 207 |
| `cargo test --features mixer` | **pass** — 208 |
| `cargo test --doc` | **pass** — 6 |
| `cargo test --features semantic` | **pass** — 362 + 36 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples` | **pass** |

Volume held **21 GB** free; no disk interruption.

## The defect: a false number in shipped documentation, found by asserting an exact one

`loudness.rs` documented that *"3 s of digital silence"* has **7** gating blocks, and the existing test's message said
*"3 s at 75% overlap is seven blocks"*. **Both are wrong.** 7 is the *non-overlapping* count (`3.0 / 0.4`); a 75 %-
overlapping block advances 100 ms, so a 3 s clip has

    1 + (144000 − 19200) / 4800 = 27

The earlier test asserted `blocks >= 7`, which **passes on 27** and therefore never noticed. The new test asserted the
exact 27 and failed with `left: 27, right: 7`.

Fixed in three places: the doc table (now says **1 s**, where 7 is correct), the existing test's bound (`assert_eq!` on
27 with the arithmetic in the message), and the new test. **A lower bound hid a wrong sentence; an exact value found it.**

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–84. `bang-spectrum.png` read: broadband onset, harmonic comb to 10 kHz, no
click line, no clipping shelf, no gap.
