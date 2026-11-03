# Iteration 76 — review

Three passes over the audio-tower verification, each with its critique written before its verdict.

## Pass 1 — Function: does it do what the source and the cue say?

**Critique.** The claim under test is narrow and numeric: *this crate's mel, feeding the `htsat-fused` audio graph,
reproduces `transformers`' embedding.* The sources name three things that could each break it — the right-hand reflect
pad, the FFT's precision, and the tiling rule — and the half-done tree had the tiling right and the other two wrong.

Measured, on the **pretrained** `laion/clap-htsat-fused` (not a default-constructed extractor, which is a different
filterbank and was the first thing this iteration got wrong):

| Quantity | Reference | This crate | Verdict |
|---|---|---|---|
| mel, frame 0, band 0 | `16.570840836` | `16.570840` | **2.4 × 10⁻⁵ dB** |
| mel, frame 1000, band 0 | `16.498333931` | `16.498333` | **2.6 × 10⁻⁵ dB** |
| mel, max abs diff over 64 064 values | — | **0.0545 dB** | inside the 0.06 dB test bound |
| mel, frames at the −100 dB floor (tiled tone) | **0** | **0** | the hole the old code had is gone |
| embedding, cosine vs the model | — | **0.999999995** | was **0.849973469** |
| embedding, max abs component | — | **2.1 × 10⁻⁵** | |
| graph alone, fed the reference's mel | — | **cosine 1.000000007** | the export is faithful; every residual is the mel |

The last row is the one that makes the rest trustworthy: it separates *my spectrogram* from *my ONNX path*, and shows
the graph is exact while the mel carries the remaining 5 × 10⁻⁸ of disagreement.

**Verdict: GREAT.** The fix moves a number the sources specify by 0.15 in cosine, and the new tests name the frames that
prove it rather than only checking a shape.

## Pass 2 — Zoom: read it closely

**Critique.** A cosine near 1.0 can hide a local defect, so the per-frame distribution was read rather than the mean.

- **The frame-0 agreement is not luck.** 16.570840836 against 16.570840 is the *same* number at `f32`'s resolution, and
  it holds across all six fixture bands (`16.544990, 16.996019, 17.427811, 17.872999, 18.320271`). A wrong window, a
  wrong filterbank transpose or a wrong dB base moves these by tens of dB; none is in play.
- **The residual is one-sided and small.** Mean absolute error **0.0013 dB** over the whole spectrogram; the largest
  single value, **0.0545 dB**, sits in a high band on a low-energy frame, which is where a `f32` output cast from `f64`
  loses its last bits. There is no frame with a *systematic* offset.
- **The old defect was a cliff, not a drift.** Before the f64 change the worst value was **11.0 dB**, and before the
  pad change the last frames fell 116 dB to the floor. Both are now absent, and `no_frame_of_the_tiled_window_falls_to_the_floor`
  asserts the count is exactly **0** — the reference's own number — so the cliff cannot come back silently.
- **No shape-only test remains.** `the_fusion_channels_are_four_windows_of_the_models_size` still checks `[4][1001][64]`,
  but it is no longer the only guard: two tests now pin numbers at frames 0, 500 and 1000.
- **A note on the earlier evidence.** `evidence/audio-tower.txt`'s claim that *"the reference's interior row sums are
  constant"* is **wrong** and is corrected here: on this tone the pretrained extractor's own row sums run
  **−3269.26 … 519.28**, because the tone is a `f32` sine whose phase resets at every tile edge. The constant-row-sum
  theory was what pointed at tiling in the first place, and it was abandoned once the real extractor was measured. The
  file is kept as history, not as proof; this review is the proof.

**Verdict: GREAT.** Every defect named in the half-done pass is either fixed and measured, or explicitly corrected as a
misreading of the reference.

## Pass 3 — Fresh eyes: a different input, and can a reader follow it?

**Critique.** Passes 1 and 2 both used the same 1 kHz tone the fixture was built from, which invites the question
"does it only agree with itself?" The crate's own **existing** single-window test — a 1 s buffer, 101 frames, the
extractor's six bands — still passes unchanged, so a *different* length exercises the same code path through a different
branch. And the 12 semantic tests include `the_same_buffer_gives_the_same_spectrogram_twice`, so determinism holds with
the new `f64` accumulation.

For a reader: `docs/guide/semantic.md` now leads with the honest state — *"the audio tower **runs** … its embedding
agrees with `transformers`' at cosine 0.850, not 1.000"* — which this iteration makes **false in the other direction**.
The page must say the tower is now verified to cosine 1.0 and name the measurement, or a reader is being told the
feature is broken when it is not. That is a doc defect this pass finds, and it is fixed before shipping.

**Verdict: GREAT**, conditional on the guide page's status line being rewritten — which it is, in the ship step.
