# Iteration 88 — research: indexing throughput, and the fixture that hid it

The item is [#36](https://github.com/bevy-coven/bevy_siren/issues/36), owed for three iterations while three costs were
added to `build` (loudness measurement, sidecar parsing, the path→hash join).

## Sources

| Source | The rule it reports |
|---|---|
| `#36`'s scope | measure **clips per second** on the target hardware, record peak VRAM and the machine's state, state the 5 000-clip projection in minutes, and **identify whether the bound is decode or inference** |
| `#36`'s technical note | *"should be a separate example rather than an extra mode on `perfcheck`"* — a per-frame cost and a batch job must never be read as the same number |
| `examples/perfcheck.rs` | the discipline this copies: print the numbers, name what they measure, say what they are not |
| `src/semantic/indexer.rs::build` | the pipeline being measured: scan → decode → downmix → resample → **measure loudness** → embed |
| `big` (the GPU host) | **RTX 3090, 24 576 MiB**, 24 cores — confirmed reachable this iteration |

## The technique, in the sources' terms

An indexer is only as fast as its slowest stage, and the two stages scale differently. Decoding is per byte and scales
with cores; inference is per clip and scales with the accelerator. **One number cannot name the bound**, so
`examples/indexbench` measures the pipeline twice:

1. **decode** — the real `build` with an embedder that returns instantly. This includes the scan, the decode, the
   downmix, the resample **and the loudness measurement**, because all of those are on the decode side of the boundary.
2. **decode + synthetic embed** — the same files with a fixed-arithmetic embedder, so the pipeline shape has a second
   stage to separate. Reported as `embed*` because it is **not the model**; its ratio to `decode` is what names the bound.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| corpus | 0.5 s mono 48 kHz 16-bit WAV, one per seed | generated, so the number is reproducible |
| synthetic embed | **262 144 multiply-adds per clip** | design default, named so nobody reads it as a model |
| decode, 200 / 500 / 1000 clips | **2 411 / 2 636 / 2 724 clips/s** | measured, M4 release |
| total (decode + synthetic embed) | **1 856 / 1 812 / 1 846 clips/s** | measured |
| 5 000 clips at this rate | **3 s** | extrapolated from the measured rate |

## What good sounds like — and the bug that nearly hid it

A bench that reports **200 indexed of 200 files**, with the same rate at 200, 500 and 1000 clips, and a projection that
is the measured rate times 5 000.

**The first run reported `indexed 24 / 24`** on a corpus of 200 files — and printed success. Two separate mistakes:

1. **The fixture lied.** `tone()` varied its frequency by `seed % 24`, so seeds 0, 24 and 48 produced **byte-identical
   files**. Measured with `shasum`: three such files share one hash. The scan, correctly, reported them as **one asset
   with several paths** — `assets 24, skipped 0, refused 0` — which is exactly what `Library::duplicated` exists for.
2. **The bench hid it.** It printed `decoded.indexed() / n` where `n` was *also* `decoded.indexed()`, so `24 / 24` read
   as complete. A ratio whose denominator is derived from its numerator cannot fail.

Fixed by making every seed's bytes unique (`hz` advancing with the seed plus a `1/√2` phase offset) and by printing
`indexed` against the **corpus size**, so the two can disagree loudly.
