# perf.md — before and after, measured this time

## The crate's per-frame work

`cargo run --release --example perfcheck`, this Mac (Apple arm64), release build 9 m 10 s after a clean.

| Voices | µs / frame | µs / voice |
|---|---|---|
| **8** | **0.024** | 0.0030 |
| **64** | **0.284** | 0.0044 |
| **256** | **0.667** | 0.0026 |

**What this measures**: `update_mix`'s work per frame as an **upper bound** — every voice is costed, including the ones
the system skips because their gain did not move. It is **not** the audio thread.

## Before and after

**No crate code on the audio path changed this iteration.** Everything added is behind `semantic` and off by default,
proven: `cargo tree --edges normal` is **413 lines** with no features, before and after. So the "before" is iteration
63's number, which was **not measured** (its `perf.md` records why: the disk was at 1.2 GB and a release build would
have failed as `ld: write() failed, errno=28`).

That gap is now closed. This iteration's figures are the baseline the next audio-path change must not exceed by more
than **10 %**:

| Voices | Baseline (2026-11-03, iteration 64) |
|---|---|
| 8 | **0.024 µs / frame** |
| 64 | **0.284 µs / frame** |
| 256 | **0.667 µs / frame** |

## A number that is suspicious, and why it is not wrong

`us/voice` **falls** from 64 voices (0.0044) to 256 (0.0026). A cost that drops as the count rises looks like a
measurement error.

It is not: at 8 voices the fixed per-frame overhead dominates, and at 256 the vector is long enough that branch
prediction and cache behaviour matter more than the per-item arithmetic. The absolute figures are also close to the
timer's resolution at 8 voices — **0.024 µs** is a fraction of a microsecond, so the low end should be read as
"below a hundredth of a microsecond", not as three significant figures.

**What a consumer can conclude**: at the crate's largest budget (256 voices) the per-frame mix costs **under
one microsecond**. That is the claim that matters, and it has margin.

## Machine state

| | |
|---|---|
| local | Apple arm64, 96 GB disk free before the release build, **4.2 GB after** |
| `target` | grew to ~12 GB with the release artefacts |
| load | one release build running; no other work on the machine |

The disk figure belongs with the numbers: the build spent ~8 GB, and a second release build would have to free space
first.

## What was not measured

The **model's** cost is not in this table and is not comparable with it — `perfcheck` measures Bevy systems, and the CLAP
query was measured separately at **5.28 ms** on an M4 CPU (iteration 63). Two different jobs, two different numbers.
