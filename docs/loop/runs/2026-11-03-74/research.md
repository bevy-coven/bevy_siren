# research.md — iteration 74: CLAP's spectrogram, in Rust, exactly

## S1 — the item, and why it needed no decision

Iteration 73's list put **audio embedding** first, noting it was *"the largest blocker that is not a decision"*. That is
what shipped: the log-mel spectrogram `ClapFeatureExtractor` produces, implemented in the crate.

## S2 — every number read from the model, not remembered

`ClapFeatureExtractor.from_pretrained("laion/clap-htsat-fused")`, printed:

| Parameter | Value |
|---|---|
| `sampling_rate` | **48 000** |
| `fft_window_size` | **1024** |
| `hop_length` | **480** |
| `feature_size` | **64** mel bands |
| `frequency_min`, `frequency_max` | **50**, **14 000** Hz |
| `mel_filters` | **`(64, 513)`**, i.e. 32,832 numbers, **band-major** |
| `nb_max_frames` | 1000 → the graph's **1001** frames |
| `top_db` | `None`, so no dynamic-range clip |

## S3 — the pipeline, read from `transformers.audio_utils.spectrogram`

Not remembered; read from the installed source, line by line:

1. **reflect-pad** by `frame_length / 2` on each side (`np.pad(mode="reflect")`, which excludes the edge sample);
2. multiply each frame by a **periodic Hann** window;
3. `rfft` of a 1024-sample buffer → **513** bins;
4. `|z|²` (`power = 2.0`);
5. `mel_filters.T · power`, floored at **`1e-10`**;
6. `10 · log10(x)` (`power_to_db` with `reference = 1.0`, `min_value = 1e-10`).

Frame count with `center: true` is `1 + len / hop`, so **48 000 samples give 101 frames** — measured against the
extractor's own `(101, 64)` output.

## S4 — the result: exact, not approximately right

A 1 kHz tone at 48 kHz, first frame, six bands:

| band | this crate | `ClapFeatureExtractor` | Δ |
|---|---|---|---|
| 0 | **16.570843** | 16.570843 | **0** |
| 1 | **16.544992** | 16.544992 | **0** |
| 2 | **16.996017** | 16.996021 | 4 × 10⁻⁶ |
| 3 | **17.427809** | 17.427816 | 7 × 10⁻⁶ |
| 4 | **17.873003** | 17.873010 | 7 × 10⁻⁶ |
| 5 | **18.320272** | 18.320274 | 2 × 10⁻⁶ |

Five of six agree to **six decimal places**; the differences are `f32` rounding in the final sum. That is not a
spectrogram that "looks right" — it is the same spectrogram.

## S5 — the layout defect, and it would have been invisible

`ClapFeatureExtractor.mel_filters` is **`(64, 513)`**, band-major. My first version assumed `(513, 64)` and applied a
transposed matrix.

**A transposed filterbank produces a spectrogram of the right shape and the wrong content** — 101 frames of 64 bands
either way, no error, no crash. It was caught only because the comparison is against the extractor's *numbers* rather
than its shape. That is the ninth time in this loop that a plausible-looking output needed a number to expose it.

## S6 — what was chosen rather than copied

`ClapFeatureExtractor`'s fusion path picks its three crop windows **at random**:

```python
idx_front  = np.random.choice(ranges[0])
idx_middle = np.random.choice(ranges[1])
idx_back   = np.random.choice(ranges[2])
```

That makes an embedding depend on a dice roll, which this crate's determinism rule forbids. [`fusion_channels`] uses
**front, middle and back** instead. For a clip that fits one window all four channels describe the same audio, so the
difference shows only on a long track — where a deterministic choice is also the reproducible one.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| window / hop / bands | 1024 / 480 / 64 | the extractor, measured |
| frequency range | 50 – 14 000 Hz | the extractor, measured |
| filterbank | 64 × 513, band-major | the extractor, measured |
| frames for 1 s | **101** | measured, and asserted |
| dB floor | **−100 dB** (`1e-10`) | the extractor's `min_value`, and asserted |
| agreement with the model | **≤ 7 × 10⁻⁶ dB** | measured, S4 |

## What good sounds like

For a spectrogram, "good" is that it is **the same spectrogram the model was trained on** — not merely one of the right
shape. The dB scale makes that testable in a way a picture cannot: five of six bands agree to six decimal places, and a
transposed filterbank, a wrong window, a missing reflect-pad or a linear rather than dB output each move the numbers by
tens. The comparison is a test, not a note.
