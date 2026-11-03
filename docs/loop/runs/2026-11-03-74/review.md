# review.md — iteration 74, three passes

## Pass 1 — Function: is it the model's spectrogram?

| Claim | Test | Result |
|---|---|---|
| the spectrogram **equals** the extractor's | `the_spectrogram_matches_the_extractors_own_output` | 5 of 6 bands to **six decimals**, worst Δ **7 × 10⁻⁶** |
| the FFT is a real DFT | `the_fft_matches_a_hand_computed_discrete_transform` | an impulse gives magnitude 1 in every bin, phase −2π/8 |
| the window is periodic Hann | `the_window_is_periodic_hann` | sum exactly n/2 |
| reflect padding is numpy's | `reflect_padding_mirrors_without_repeating_the_edge` | `[3,2,1,2,3,4,3,2]` |
| 1 s → 101 frames | `one_second_of_audio_gives_the_frame_count_the_pipeline_gives` | **101**, matching the extractor's `(101, 64)` |
| silence is the floor | `a_silent_buffer_sits_at_the_floor_not_at_negative_infinity` | **−100.0 dB** |

**The visual gate is unchanged**, which is the check that this iteration did not disturb the audio path: 32 render
files, **14 distinct** cue hashes, **−21.5 LUFS**, **−7.5 dBFS**, the duck recovering by **1.97 s** — all identical to
iteration 67.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Two defects, and the first is the one worth remembering:

| Defect | Found by | What it was |
|---|---|---|
| the filterbank was assumed **`(513, 64)`**; the extractor stores **`(64, 513)`** | the numbers, not the shape | a transposed matrix gives a spectrogram of the **right shape and wrong content** — 101×64 either way, no error |
| `for bin in 0..BINS` indexed `power` | clippy | zip instead, which is also the shape the compiler vectorises |

The first is **the ninth time in this loop a plausible output needed a number to expose it.** A spectrogram of the right
shape is exactly the kind of result that ships: it has the dimensions the graph wants, the dB range looks sensible, and
every downstream comparison is quietly wrong.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **The fixture is the model's own data**, checked in as `tests/fixtures/clap-mel-filters.txt` — 71 KB of text, 32,832
  numbers — so the comparison cannot drift and needs no Python to re-run.
- **The module doc says exactly where each constant came from**, so a model upgrade is a visible change rather than a
  mystery.
- **The one place it deliberately differs is named**: `fusion_channels` uses fixed front/middle/back windows where the
  original chooses at random, because a randomised crop makes an embedding depend on a dice roll.
- **The default build is untouched**: 198 tests with no features; all of this is behind `semantic`.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | CLAP's log-mel spectrogram, in Rust, and `Model::embed_audio` |
| new module | `src/semantic/mel.rs` |
| tests | **+10** lib tests (unit, inside the module) |
| agreement with the model | **≤ 7 × 10⁻⁶ dB** |
| visual gate | unchanged: −21.5 LUFS, −7.5 dBFS, 14 distinct cues |
| carved off | running the audio tower against the real 116 MB graph (needs `ort` on `bmb`) |
