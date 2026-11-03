# perf.md

## The crate's per-frame work

**Not re-measured this iteration, and the reason is exact:** no code on the audio path changed. Everything added is
behind `semantic`, proven by `cargo tree --edges normal` reading **413 lines** with no features, before and after.

The baseline iteration 64 established stands:

| Voices | µs / frame | µs / voice |
|---|---|---|
| 8 | **0.024** | 0.0030 |
| 64 | **0.284** | 0.0044 |
| 256 | **0.667** | 0.0026 |

## The conversion's cost, which is new work and is not in that table

The resampler runs **offline, per asset**, not per frame. It was not timed as a throughput figure because `P-01` owns
that measurement and needs the 3090; what is recorded is that it **converges**: 1.000 s of 44.1 kHz audio becomes
48000 samples of 48 kHz audio in the crate's test suite, which runs **282 tests in 2.9 s**.

## Machine state

| | |
|---|---|
| local | Apple arm64, started at **1.6 GB free**, `target` 23 GB |
| after `cargo clean` | **23 GB free** |
| after the release build and render | **221 MB free** |

The disk figure belongs with any future number: a release build plus a render spends ~20 GB here, and a second
release build would have to free space first. A full disk fails as `ld: write() failed, errno=28`, which looks like a
code error and is not one.
