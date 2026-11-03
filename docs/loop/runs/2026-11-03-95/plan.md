# Iteration 95 — plan: one transform per frame

## Ranked items

1. **Share the spectral pass between `spectral` and `tempo`** — chosen. Measured waste, not suspected: 4.33 ms per 4 s
   clip.
2. Share with `mel::log_mel` — **not this iteration**, and the reason is in `research.md`: `log_mel` reflect-pads and
   derives its frame count from the padding, so its frames are not the same frames. Sharing it is a change to *what is
   computed*, not to *how often*, and needs its own evidence.
3. `#40` weighted ensemble with a per-result breakdown — the largest open retrieval item, and the one that would make the
   cost worth paying.
4. `#45` regression harness (recall@k) — needs `#35`'s golden set first.
5. The `#28`–`#31` tool surface.

**Why 1 beats 2:** item 1 removes work that is already being done twice, with a test suite that proves the answer did not
move. Item 2 changes the answer, so it must justify a different one.

## Design

New module `src/semantic/analysis.rs`:

- `pub struct Spectra { summed_power: Vec<f64>, frame_energy: Vec<f32>, frame_len: usize }`
- `pub fn spectra(buffer: &[f32]) -> Spectra` — one FFT per frame, `re`/`im` allocated once outside the loop.
- `pub fn onset_envelope(frame_energy: &[f32]) -> Vec<f32>` — the positive frame-to-frame rise.

Both consumers gain a `*_spectra` entry point that takes the precomputed value:

- `spectral::analyse_spectra(buffer, &Spectra)`; `spectral::analyse` is now `analyse_spectra(buffer, &spectra(buffer))`.
- `tempo::estimate_spectra(buffer, &Spectra)`; `tempo::estimate` is its unshared form.

`indexer::embed_measured` computes `spectra(&mono)` once and passes it to both.

## Fields

None. No rate, curve or threshold changes, so no citation and no design default is owed.

## Tests

- `analysis`: the frame count matches what both callers already used (`1 + (len − WINDOW)/HOP`, and one frame below a
  window); a 1 kHz tone's power peaks in its own bin ±1; silence gives zero, not NaN; an onset envelope keeps only rises;
  the same buffer gives the same spectra twice.
- **The load-bearing test is that the existing 10 spectral and 13 tempo tests pass with no edits to their expectations.**
- A/B measurement in `perf.md`: the shared pair against the separate pair.
