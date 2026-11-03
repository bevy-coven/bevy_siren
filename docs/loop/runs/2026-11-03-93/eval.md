# Iteration 93 — eval

## The cases

`src/semantic/` only, behind the non-default `semantic` feature.

| Case | Green/red | Observation |
|---|---|---|
| 1–33 | **green** | `cargo test` **198**; `--features serde` **207**; `--features mixer` **208**; doc **6** |
| semantic suite | **green** | **380** (376 + 4 new) and **80** lib |
| 19 (per-frame) | **green** | 0.323 / 1.119 µs/frame at 64/256 |
| tempo produced by a build | **green, new** | a click track stores **120 BPM** with no caller supplying it |
| key produced by a build | **green, new** | a click train stores **no key** — a record, not a gap |
| both round-trip in both formats | **green, new** | including the two absences |
| no section when empty | **green, new** | every earlier index is byte-identical and loads |

## Baseline

| Step | Result |
|---|---|
| `cargo build` / `test` / `--features serde` / `--features mixer` / `--doc` | **pass** — 198 / 207 / 208 / 6 |
| `cargo test --features semantic` | **pass** — 380 + 80 |
| `cargo clippy --all-targets --all-features -- -D warnings` | **pass**, 0 warnings |
| `cargo fmt --check` | **pass** |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | **pass** |
| `cargo build --examples --features semantic` | **pass** |

Volume held **19 GB** free; no disk interruption.

## Two defects, both in my assertions

1. **`a_clip_too_short_to_analyse_still_embeds` asserted the wrong thing.** I claimed a 0.1 s clip would have *no tempo
   record*; it has one, because 4 800 samples is longer than tempo's two 1 024-sample frames — it simply has no `bpm`.
   Only **key** skips the record, since 4 800 is shorter than its 8 192-sample chroma window. The test now asserts all
   three states: a tempo record with `bpm: None`, no key record, and a vector either way.

   That is the **sixth** fixture-shaped error in this loop, and the first where the fix was to assert *more* rather than
   differently — the three states are the feature.

2. **`Key` has no confidence field**; `Estimate` carries it. My first write called `k.confidence_of()` on a `Key`, which
   does not exist — the type was right and the field I reached for was in the wrong struct. Storing the `Estimate` is
   also the better design, since `#27` may want the confidence.

## Visual gate

| Measure | Value |
|---|---|
| render files | **32** |
| distinct cue hashes | **14 of 14** |
| master integrated | **−21.5 LUFS** |
| master true peak | **−7.5 dBFS** |

Identical to iterations 67, 71, 74, 75, 80–92.
