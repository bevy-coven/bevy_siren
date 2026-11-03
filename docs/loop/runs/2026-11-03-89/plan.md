# Iteration 89 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#22` spectral centroid, rolloff, noise floor** | p1, unblocked by ADR 0003, and it **blocks `#27`** — the hard filters that are the consumer of everything built since iteration 77. |
| 2 | `#28` the MCP tool host | Largest user-facing win. |
| 3 | Finish `#36` on the GPU | Needs the current commit and the graph on `big`. |
| 4 | `#27` hard filters | Blocked on this. |
| 5 | `f16` with `V-01` | Needs a labelled set. |

**Why (1) beats (2) in one sentence:** #27 is the layer that finally *uses* loudness and spectral shape, and it cannot
start until the second of those exists.

## The change

`semantic::spectral` — `analyse(&[f32]) -> Result<Spectral, SpectralProblem>`, plus a third optional index section.

| Decision | Choice | Why |
|---|---|---|
| silence | centroid/rolloff **`None`** | `#22`'s own pitfall: a fully silent clip divides by zero. `0 Hz` is a *real* reading for a DC offset, so a sentinel would conflate them |
| a non-finite sample | **refused, located by index** | "a NaN is worse than a wrong number" — it makes every comparison false |
| spectra summed as | **power**, not magnitude | the centroid is defined over energy |
| short clip | one frame, not padded | padding a 20 ms click to 1024 samples dilutes every statistic |
| rolloff fraction | **0.95**, exported | `#22` requires the percentage be stated |
| noise floor | quietest **10 %** of frames | **design default**; the method is named so a reader can disagree precisely |
| the FFT | `mel::fft_in_place`, `pub(crate)` | the same transform tested to 0.0545 dB against CLAP |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/spectral.rs` | **new** — `Spectral`, `SpectralProblem`, `analyse`, `ROLLOFF_FRACTION`, 10 tests |
| `src/semantic/mod.rs` | one `pub mod` |
| `src/semantic/mel.rs` | `hann` and `fft_in_place` become `pub(crate)` |
| `src/semantic/index.rs` | a `spectral` section in both formats, `hertz_field`, `parse_spectral_line` |
| `src/semantic/indexer.rs` | `Analysed` struct; `embed_measured` returns it; `build` stores the features |
| `tests/semantic_index.rs` | **3 new tests** |

## Tests that prove it

1. `a_sine_centroid_is_its_own_frequency` — 220 / 1000 / 4000 / 8000 Hz, each within **5 %** (criterion 1).
2. `a_low_passed_copy_is_darker_than_white_noise` — centroid **and** rolloff lower (criterion 2).
3. `digital_silence_has_the_crates_floor_and_no_centroid` — criterion 3, and `Spectral::absent()`.
4. `silence_never_produces_a_nan` — the pitfall, from both `0.0` and `-0.0`.
5. `spectral_features_round_trip_in_both_formats` — criterion 4, including that silence and a DC offset stay distinct.
6. Eight more: short clip, DC offset, non-finite sample, empty, determinism, rolloff-with-brightness.

## Carved off

- **Perceptual weighting** (`#22`'s own "out"): no A-weighting, stated as absent rather than silently implied.
- **Optimising the cost.** The regression is measured and reported; making it cheaper is a separate, measured task.
