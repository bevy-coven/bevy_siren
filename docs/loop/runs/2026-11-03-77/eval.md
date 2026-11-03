# Iteration 77 — eval

Two items shipped this iteration: the **ADR (#4)** and the **loudness measurement (#21, partially)**.

## The cases, and what changed

`src/semantic/loudness.rs` and `docs/` are behind the **non-default** `semantic` feature or are documents. The audio
path is untouched:

| Check | Result |
|---|---|
| files changed outside `src/semantic/` | `Cargo.toml` (optional dep), `tests/leaf.rs` (the ratchet's doc + entry), `CHANGELOG.md`, `docs/` |
| audio-path files changed (`model.rs`, `cue.rs`, `mix.rs`, `play.rs`, `music.rs`, …) | **0** |
| `semantic` in `default = []`? | **no** |
| cases 1–33 (audio behaviours) | **unchanged** — same sources, same tests |

## Baseline

Disk was at **100 %, 122–230 MB free** for the whole iteration. This is the same trap iteration 76 recorded, and it was
worse here: `cargo test` and every all-features step fail with `No space left on device (os error 28)` or
`rustc-LLVM ERROR: IO failure on output stream`. Reclaiming via `cargo clean -p` bought ~1.3–1.7 GB, which the next
linker invocation consumed again.

| Step | Result |
|---|---|
| `cargo test --lib --features semantic` | **pass — 20 tests**, including 8 new loudness tests |
| `cargo fmt --check` | **pass** |
| `cargo build --features semantic` | **pass** (earlier in the run) |
| `cargo build` | **pass** (earlier in the run) |
| `cargo test` (default) | **BLOCKED — disk**, `could not compile bevy_pbr/bevy_app/bevy_gizmos_render` |
| `cargo test --features serde` / `mixer` / `doc` | **BLOCKED — disk** |
| `cargo clippy --all-targets --all-features` | **BLOCKED — disk** |
| `cargo doc`, `cargo build --examples`, `perfcheck`, `examples/render` | **BLOCKED — disk** |

**Why the default suite cannot have broken.** `semantic` is off by default; `ebur128` is `optional = true` and reachable
only through `semantic = [..., "dep:ebur128"]`; `tests/leaf.rs`'s change is a doc comment plus one name in
`ALLOWED_DEPS`, and the test that enforces that list also asserts every name is present in the manifest — which the
`semantic` build satisfies. But **unrun is unrun**, and the next iteration must run the full baseline once the disk is
cleared.

## The loudness numbers, against the reference

| Clip | `ffmpeg` reference | `semantic::loudness` | Test |
|---|---|---|---|
| 3 s 1 kHz, 0.1 amplitude | −23.0 LUFS, −20.0 dBFS | **−23.0 LUFS** (asserted ±0.1) | `a_tone_matches_the_references_integrated_loudness` |
| 3 s 0.5 vs 0.1 amplitude | 13.98 dB apart | **13.98 dB** (±0.1) | `five_times_the_amplitude_is_fourteen_decibels_louder` |
| **0.2 s**, 0.5 amplitude | −70.0 LUFS (the floor as a level) | **`None`**, peak −6.02 dBFS | `a_clip_shorter_than_a_gating_block_has_no_integrated_loudness` |
| 400 ms exactly | one block | **1 block, loudness present** | `the_short_clip_boundary_is_where_the_standard_puts_it` |
| 3 s digital silence | −70 LUFS | **`None`, `silent() == true`, `too_short() == false`** | `silence_measures_at_the_floor_rather_than_being_absent` |

## The defect this iteration found in its own first draft

The first version carried a single `blocks` field and derived `too_short()` from `integrated.is_none()`. Its own silence
test failed and showed why: `ebur128`'s `gating_block_count_and_energy` counts blocks **above the absolute gate**, so
3 s of silence and a 0.2 s footstep both returned **0 blocks**. The module was conflating *"too short to measure"* with
*"genuinely silent"* — the exact error it was written to prevent, reproduced one level down.

Fixed by carrying **both** counts: `blocks` (above the gate) and `blocks_possible` (allowed by length), with `too_short()`
and `silent()` as separate predicates. Both are now asserted, in opposite directions, on the two clips.

## Not done, and why

- **The index round trip.** #21's remaining criterion needs a schema change to the versioned `bevy_siren index 1` text
  format, which the crate's rules send to `docs/loop/needs-decision/loudness-in-the-index.md` (recommendation: an
  optional `loudness` section, because absence must not become a sentinel number).
- **The perf and render gates.** Both need a release build; neither is measurable at 124 MB free.
