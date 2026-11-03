# Iteration 89 — research: spectral shape, and three features that cost more than they should

The item is [#22](https://github.com/bevy-coven/bevy_siren/issues/22) — spectral centroid, rolloff and noise floor —
unblocked by ADR 0003 and now cheap to cost, because iteration 88 built the bench that measures what they add.

## Sources

| Source | The rule it reports |
|---|---|
| `#22` | centroid and rolloff **with the percentage stated**; a noise-floor estimate **with its method named**; the analysis window and short-clip handling stated; a **NaN is worse than a wrong number**, and a silent clip divides by zero in a centroid |
| `src/semantic/mel.rs` | `WINDOW = 1024`, `HOP = 480`, `BINS = 513`, `MEL_FLOOR = 1e-10` — the constants reused here so a spectral number and a mel frame describe the same slice |
| `src/semantic/mel.rs::fft_in_place` | a radix-2 transform in `f64`, tested against CLAP's extractor to **0.0545 dB** (iteration 76) |
| ADR 0003 | the crate's rule that a feature's absence is a state, not a sentinel — applied here to silence |
| `runs/2026-11-03-88` | the decode-side baseline these features are measured against: **2 636 clips/s** at 500 clips |

## The technique, in the sources' terms

All three statistics come from **one** pass over the frames, and the pass is the mel path's: a Hann-windowed 1024-sample
frame, hopping 480, transformed with the same radix-2 FFT. Spectra are summed as **power**, not magnitude, because the
centroid is defined over energy: summing magnitudes weights a steady tone by amplitude, and a loud low tone would then
outrank a quiet bright one that carries more energy.

- **Centroid** = `Σ(bin · power) / Σ power`, in Hz.
- **Rolloff** = the lowest bin whose cumulative share reaches **0.95** of the total — the standard MIR choice, exported
  as a named constant so a caller knows which percentage a number used.
- **Noise floor** = the mean power of the **quietest tenth of frames**, in dB against the clip's own loudest frame. The
  quiet tenth rather than the minimum because one silent frame between two loud ones is a gap, not a floor.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| window / hop | **1024 / 480** | `mel::WINDOW`, `mel::HOP` |
| rolloff fraction | **0.95** | the MIR standard; exported as `ROLLOFF_FRACTION` |
| noise-floor percentile | **the quietest 10 %** | **design default** — no source fixes a percentile |
| short clip | analysed as one frame, **not padded, not refused** | `#22`'s "how a clip shorter than it is handled" |
| silence | centroid `None`, rolloff `None`, floor `SILENT_DB` | `#22`'s NaN rule |
| DC offset | centroid **9.375 Hz**, not 0 | measured — see below |

## What good sounds like — and the two things the tests taught

A 1 kHz sine reads a centroid within 5 % of 1 000 Hz; white noise reads brighter than its low-passed copy; digital
silence reads `SILENT_DB` with **no** centroid rather than `0 Hz` or NaN.

**Two assertions were wrong before the code was:**

1. **A DC offset does not read 0 Hz.** Multiplying a constant by a Hann window produces a raised cosine, whose spectrum
   is three bins wide rather than an impulse at bin 0, so the energy-weighted mean sits at **9.375 Hz**. The first test
   asserted `Some(0.0)` and failed. The measured value is now asserted, with the leakage explained — a DC offset is
   *darker than anything audible* either way, and pretending it is exactly zero would hide the window's behaviour.
2. **A "short" clip must actually be short.** The first fixture used 0.1 s, which is **4 800 samples — nearly five
   windows**, so the short-clip branch was never exercised. The window is **21.3 ms**; the fixture is now 10 ms.

**And a cost that is larger than expected**, stated because it is a regression: decode fell from **2 636 to ~1 150
clips/s** at 500 clips. The spectral pass is a second full FFT sweep over frames the mel path does not compute for the
embedder (which uses CLAP's own preprocessing), so it roughly doubles the decode side.
