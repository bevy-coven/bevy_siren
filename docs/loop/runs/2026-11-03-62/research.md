# research.md — iteration 62, the semantic critical path

This iteration is engineering against a decided backlog (#2–#49), not a design search. The evidence is a measurement,
so the "source" is the model itself and the numbers are the spike's output.

## S1 — `laion/clap-htsat-fused`, measured on `bmb` (Apple M4, torch 2.15.0, transformers 5.22.1)

| Quantity | Measured |
|---|---|
| total parameters | **153,507,530** |
| audio tower parameters | **27,549,128** |
| text tower parameters | **124,645,632** |
| processor keys | `input_features`, `is_longer` |
| mel input shape (padded, 3 clips) | `[3, 4, 1001, 64]` |
| audio embedding shape | `[3, 512]` |

**The technique.** `ClapModel.get_audio_features` is exactly three steps, read from the installed source at
`transformers/models/clap/modeling_clap.py`:

```python
audio_outputs = self.audio_model(input_features=..., is_longer=...)
audio_features = self.audio_projection(audio_outputs.pooler_output)
audio_outputs.pooler_output = F.normalize(audio_features, dim=-1)
```

`is_longer` enables HTSAT's **feature fusion**: when the clip is longer than the model's 10 s window, a second,
downsampled spectrogram branch is concatenated. That is the branch `I-03`'s chunker must reason about, and it is a
per-batch tensor, not a per-call flag.

**The last line matters most for this crate.** The embedding is **L2-normalised** before it is returned. So a cosine
similarity between two CLAP embeddings is already a dot product of unit vectors, and the crate must not normalise
again — or must normalise knowingly. `R-01`'s notes say "normalise at write time and state it"; the measurement says
the model already did, so the indexer normalises once and records it.

## S2 — the ONNX export, measured

| Quantity | Measured |
|---|---|
| export (torchscript, opset 17) | **ok** |
| file size | **116.5 MB** |
| graph nodes | 3,670 |
| distinct operators | 39 |
| `onnx.checker` | **pass** |
| cosine vs PyTorch, batched 10 s | **1.000000000** |
| max abs difference | **6.8 × 10⁻⁷** (fp32 epsilon) |

The 39 operators are all standard: `Conv`, `Gemm`, `MatMul`, `LayerNormalization`, `Resize`, `ScatterND`, `Softmax`,
`Erf`, `ReduceL2` and the shape arithmetic. **No custom domain, no `ai.onnx.contrib`, no fallback operator.**

## S3 — variable length, measured

| Clip | Mel frames | Cosine | Max abs diff |
|---|---|---|---|
| 1 s | 1001 | **1.000000000** | 6.4 × 10⁻⁷ |
| 4 s | 1001 | **1.000000000** | 4.1 × 10⁻⁷ |
| 10 s | 1001 | **1.000000000** | 6.8 × 10⁻⁷ |

**The `time` axis in `input_features` is a red herring.** All three durations produce **1001 mel frames**, because the
processor resamples and pads to the model's fixed window. So CLAP's audio input is *fixed-shape in time* and *variable
only in batch*. That removes `I-03`'s hardest case from the CLAP path and puts it where it belongs: the chunker decides
how many 10 s windows a track becomes, not how to shape one tensor.

## S4 — the one real defect: dynamic batch

| Batch requested | Result |
|---|---|
| 1 | **ok** |
| 2 | **FAIL** — `Reshape` at `/audio_model/audio_encoder/patch_embed/Reshape_5` |
| 8 (dynamic export) | **FAIL**, same node |
| 8 (exported **at** batch 8) | **ok**, cosine **1.000000000**, 6.0 × 10⁻⁷, 116.5 MB |

`patch_embed` reshapes a 4-D feature map to a 3-D sequence and the tracer baked the batch axis into the reshape's
target shape. Declaring `dynamic_axes` for the batch dimension does not fix it, because the constant is inside the
graph's shape arithmetic rather than at an input.

**The consequence for `N-02`.** Batching is possible, but **one file per batch size**, or a re-export per size. That is
a real cost and must be a stated decision rather than discovered in production: either export a small ladder
(1, 8) and dispatch, or run batch 1 and accept the throughput. `N-02` cannot assume a dynamic batch.

## S5 — latency, measured

| Device | Batch | Per clip |
|---|---|---|
| **Apple M4 CPU** (`CPUExecutionProvider`) | 1 | **34.9 ms** |

This is a *CPU* figure on a laptop, with no CUDA and no `fp16`. On the RTX 3090 the audio tower is 27.5 M parameters —
small enough that the figure should be dominated by call overhead rather than arithmetic. `P-01` should re-measure
rather than extrapolate.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| CLAP audio tower | 27.5 M parameters | measured, S1 |
| CLAP text tower | 124.6 M parameters | measured, S1 |
| embedding dimension | **512** | measured, S1 |
| audio window | fixed 1001 mel frames | measured, S3 |
| export size, audio tower only | 116.5 MB | measured, S2 |
| fp32 export agreement | 6.8 × 10⁻⁷ max abs | measured, S2 |

## What good sounds like

For a *vector*, "good" is numeric agreement: cosine ≥ 0.999 against the reference on every clip. **Measured 1.000000000
across all of them.** The ranking quality that follows is `V-02`'s question, not this one's.
