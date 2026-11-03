# Iteration 95 — perf: one transform per frame

Machine: this Mac, shared, 96 % of a 461 GB volume used at the time of the run (disk fullness affects link time, not these
figures). All figures are **the crate's own work**, released build, `--features semantic`.

## The direct A/B — what the change actually does

40 repetitions of each shape on a 4 s synthetic click track at 48 kHz:

| shape | ms per call |
|---|---|
| `spectral::analyse` + `tempo::estimate` (separate passes) | **9.44** |
| `analysis::spectra` once, fed to `analyse_spectra` + `estimate_spectra` | **5.11** |
| saved | **4.33 ms — 45.9 %** |

The saving is almost exactly one 1024-point FFT pass over 400 frames, which is what the duplication was.

## Per-feature, before and after

Measured by calling each feature on its own (so each still pays for its own pass — this row is **not** the shared path):

| feature | iteration 94 | now |
|---|---|---|
| loudness | 2.40 ms | 2.40 ms |
| spectral | 3.96 ms | 4.19 ms |
| tempo | 4.23 ms | 4.16 ms |
| key | 4.69 ms | 4.81 ms |

Those are within run-to-run noise of each other and **are not the claim**. The claim is the table above: in `build`, where
both are asked for, only one pass happens.

## End to end, which is the number that matters

`examples/indexbench`, 300 clips, three runs, after:

| run | decode |
|---|---|
| 1 | 766.2 clips/s |
| 2 | 784.0 clips/s |
| 3 | 698.6 clips/s |

Against iteration 93's **419.8 / 568.8 clips/s** for the same benchmark shape: **median 766 against median 512, a 40 %
recovery** in decode throughput, which is consistent with the 45.9 % removed from the analysis pair.

`5000 clips at this rate: 9 s`.

## Per-frame audio cost

`perfcheck` unchanged in shape: 64 voices 0.327 µs/frame, 256 voices 1.132 µs/frame, 0.0051 / 0.0044 µs per voice. The
change is in the offline indexer and touches no system that runs per frame, so no case it did not target was slowed —
there is nothing in the runtime path to slow.

## What was not fixed

`mel::log_mel` still computes its own transform. It reflect-pads to a frame count derived from the padding, so its frames
are not these frames; sharing it would change the answer rather than the cost. It is the next item, and it is bigger.
