# Iteration 76 — the audio tower, verified against its reference

Iteration 75's first item: *"Run the audio tower against the real graph on `bmb`. It is now the only unverified link in
the chain."* This iteration ran it, and the run found the link was **wrong**, not merely unverified.

## Sources

| Source | Number it reports |
|---|---|
| `transformers` 4.57.6, `models/clap/feature_extraction_clap.py` | `padding="repeatpad"`, `truncation="fusion"`, `nb_max_samples=480000`, `hop_length=480`, `fft_window_size=1024` |
| `transformers/audio_utils.py::spectrogram` | `np.pad(waveform, (frame_length//2, frame_length//2), mode="reflect")`; **promotes the waveform to float64** before `np.fft.rfft`; `num_frames = 1 + floor((size - frame_length)/hop)`; `num_frequency_bins = fft_length//2 + 1` |
| `spectrogram`'s `power_to_db` doc | `min_value = 1e-10` on a **power** spectrogram is −100 dB; `log10` is `10·log10(x)` |
| `ClapFeatureExtractor._get_input_mel`, smaller-than-window branch | `n_repeat = int(max_length / len(waveform))`, `np.tile(waveform, n_repeat)`, then `np.pad(..., mode="constant")` to `max_length`; then **`np.stack([mel] * 4)`** |
| `laion/clap-htsat-fused` — 27,549,128 audio-tower parameters, 153,507,530 total | the checkpoint this crate targets |
| vendored `tests/fixtures/clap-mel-filters.txt` | 513 × 64 = 32,832 weights, the **transpose** of `(64, 513)` band-major — verified exact against the pretrained extractor in this iteration |

## The technique, in the source's terms

For audio **shorter** than one window, `ClapFeatureExtractor` does not pad the *spectrogram*; it pads the *waveform*. It
tiles the waveform `int(max_length/len)` times, zero-pads the tail to 480 000 samples, computes **one** log-mel over the
whole thing (1001 frames), and stacks that single spectrogram four times, because `htsat-fused`'s fusion branch expects
four channels even when it is not fusing.

The mel itself is `spectrogram(waveform, hann(1024), frame_length=1024, hop_length=480, power=2.0, mel_filters,
log_mel="dB")`: reflect-pad 512 samples **on each side**, frame at `n·hop` over the padded buffer, window, `rfft`,
`|z|²`, `mel_filters.T · power` floored at `1e-10`, `10·log10`, then transpose to `[frames][bands]`.

## Parameters the sources report

| Parameter | Value | Where |
|---|---|---|
| window / hop | 1024 / 480 | extractor |
| mel bands | 64, range 50 – 14 000 Hz, htk scale, `norm=None` | extractor |
| dB floor | 1e-10 on power → −100 dB | `power_to_db` |
| frames per window | 1001 | `1 + 480000/480` |
| right reflect pad | **512 samples**, not `needed − len` | `np.pad((512, 512))` |
| FFT precision | **`np.float64`** | `waveform.astype(np.float64)` |
| reference embedding (1 kHz tone, 0.5 amplitude, 48 kHz) | norm 0.99999994, first 6 `[-0.028341768, 0.056292962, -0.022673305, -0.016399540, -0.030619279, -0.001261312]` | measured this iteration on the pretrained model |

## What good sounds like

A correct port reproduces the reference **numerically**, not by shape: frame 0 of the mel agrees to 2.4 × 10⁻⁵ dB, the
worst of 64 064 values to 0.054 dB, no frame of a tiled tone sits at the −100 dB floor, and the 512-d embedding reaches
**cosine 1.000** against the model's own. A wrong right-hand pad does not look wrong — it produces the right shape,
right norm, and plausible numbers — and shows up only as holes at the far end of the spectrogram and a cosine around
0.85. That is why the test has to name a *frame index near the end*, not frame 0.
