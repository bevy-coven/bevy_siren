# 0002 — Inference runtime: `ort`, and why the sidecar stays a fallback

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#3](https://github.com/bevy-coven/bevy_siren/issues/3)
- **Evidence:** [#2's spike](../../loop/runs/2026-11-03-62/evidence/spike/clap-export.json)

## The question

Which runtime runs the retrieval models: a Rust-native inference library, or a Python sidecar? And what is the
promotion rule for the fallback?

## What was measured first

[#2](https://github.com/bevy-coven/bevy_siren/issues/2) exported CLAP's audio tower and ran it in `onnxruntime` on an
Apple M4. The numbers that decide this ADR:

| | |
|---|---|
| export | succeeded, torchscript exporter, **opset 17**, `dynamo=False` |
| file size | **116.5 MB** |
| operators | **39 distinct, all standard** — no custom domain |
| `onnx.checker` | pass |
| cosine vs PyTorch | **1.000000000**, max abs diff **6.8 × 10⁻⁷** |
| latency, M4 **CPU**, batch 1 | **34.9 ms per clip** |
| dynamic batch | **broken** at batch ≥ 2; a fixed-batch export is exact |

## Decision

**`ort` with the CUDA execution provider, opset 17.** The Python sidecar is not built unless a later measurement
promotes it.

The reasoning is the measurements, not a preference:

- **The graph is ordinary.** 39 standard operators, `onnx.checker` passing, and exact agreement with PyTorch. The
  export risk this ADR was written to hedge against did not materialise for the audio tower.
- **`candle` was rejected on coverage, not taste.** Reimplementing HTSAT's feature fusion and its `patch_embed`
  reshape in a partial operator set is a second implementation of the thing that was just proven exact. `candle` is the
  fallback if `ort`'s build story fails, not the first choice.
- **34.9 ms per clip on a laptop CPU** means the model is small enough that the runtime's overhead matters as much as
  its arithmetic. `ort` has the smaller overhead of the Rust-native options.

## The batch constraint, recorded as a constraint

The spike proved dynamic batch does not work: `patch_embed` bakes the batch axis into a `Reshape` target shape, and
declaring `dynamic_axes` does not reach it.

**Decision for [#18](https://github.com/bevy-coven/bevy_siren/issues/18): export a ladder of batch sizes (1 and 8) and
dispatch on what is left to embed**, rather than assuming one graph accepts any batch. Two files of 116.5 MB is 233 MB,
which is acceptable; a third size needs a measurement to justify it.

## The fallback, and the exact trigger that promotes it

A Python sidecar is built **only if** one of these is measured, not predicted:

1. `ort` cannot be built on a target the crate supports (a platform's CUDA coupling or a linker failure that has no
   workaround), **and** `candle` cannot run the graph.
2. A model that `S-05` approves has operators outside `ort`'s coverage that cannot be replaced.
3. VRAM on the target device cannot hold the models `ort` loads, where a sidecar's process boundary would help — this
   is the weakest trigger, because a process boundary does not create VRAM.

**Not a trigger:** latency. 34.9 ms on a CPU with no optimisation is already fast enough to query with, and a sidecar
adds a transport hop rather than removing one.

## What the build actually needs, measured

`runs/2026-11-03-63/evidence/ort-probe.txt` ran `ort 2.0.0-rc.15` against the exported CLAP text graph:

| | |
|---|---|
| build, macOS arm64 | **3.2 s** with `["std", "ndarray"]` |
| then linking | **fails** — undefined ONNX Runtime symbols |
| adding `download-binaries` alone | **fails** — *"a TLS feature must be configured"* |
| `["std", "ndarray", "download-binaries", "tls-rustls"]` | **8 s**, and ONNX Runtime 1.31 is fetched |
| loads the 501 MB text graph | **yes** |
| reports dynamic axes | **yes** — `input_ids: [-1, -1]`, symbols `["batch", "tokens"]` |
| returns an embedding | **yes**, L2 norm **1.000000** |

**Two features, not one.** This is recorded because it is the kind of thing a consumer adds one of, gets a build error
from, and gives up on. `download-binaries` fetches the runtime; `tls-rustls` is what lets the build script make the
request.

**API churn is a real cost.** Between the published docs and this release, the constructor moved to
`ort::environment::init().build()`, `Session::builder` began requiring an `&Environment`, and the tensor extractor
returns a `(shape, data)` pair. Every one of these was found by the compiler, not the changelog. Pinning the exact
version — `=2.0.0-rc.15` — rather than a caret range is deliberate for a release candidate.

**The default build is unaffected:** `cargo tree --edges normal` is **413 lines** with no features and **424** with
`--features semantic`, and `ort` appears in exactly one of them.

## Cost to a consumer

| | |
|---|---|
| dependency | `ort` plus a downloaded ONNX Runtime shared library |
| binary size | the runtime, plus the model files a project downloads |
| MSRV | must stay at or below this crate's floor of 1.97.1 — verified in `S-09` |
| platform | macOS arm64 and Linux x86_64+CUDA are the two that must build |

A user who does not enable the `semantic` feature sees **no** change: the dependency is behind a non-default feature and
`cargo tree` with no features must be identical to 0.1.0's.

## What would reverse this

A model whose ONNX export is not exact, or whose licence audit ([](https://github.com/bevy-coven/bevy_siren/issues/6))
leaves only a PyTorch-only option. Both are measured questions, not arguments.
