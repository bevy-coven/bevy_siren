# Iteration 121 — perf

**The machine**: Apple M4 Pro, macOS 26.6.2, a shared interactive session (load average 1.8 – 2.9 during the runs, 4 users), release profile. **What is measured**: `perfcheck` is the crate's per-frame work (`update_mix` and its neighbours), an **upper bound** — it costs every voice, including the ones `update_mix` skips because their gain did not move — and **not** the audio thread's.
**What this change can cost**: `Rng::new` runs when a stream is built (a seed change, or the first frame) and `splitmix64` is one add, three xor-shifts and two multiplies. The draws (`next_u32`) are **unchanged**. The loop `perfcheck` times draws nothing.

## `cargo run --release --example perfcheck`, eight runs (`evidence/after/perfcheck.txt`)

| voices | this iteration, eight runs (µs/frame) | median | iteration 120 (three runs) | iteration 119 (three runs) | change |
|---|---|---|---|---|---|
| 8 | 0.041 – 0.067 | 0.060 | 0.061 – 0.062 | 0.057 – 0.062 | none |
| 64 | 0.246 – 0.374 | 0.333 | 0.322 – 0.357 | 0.304 – 0.359 | none |
| 256 | 0.769 – 1.248 | 1.170 | 0.837 – 1.152 | 1.053 – 1.225 | none |

The spread **inside this iteration's own runs** (0.77 – 1.25 µs at 256 voices: ±24 %) is wider than the difference to any earlier band, and every earlier band lies inside it. No case moves by 10 % or more that the run-to-run noise of a shared machine does not also move by. Nothing `perfcheck` times changed; this is the check that it did not.

## `Rng::new`, `splitmix64` and a draw, timed directly (a scratch release program, deleted; three rounds)

| | ns | notes |
|---|---|---|
| `Rng::new(seed)` | 0.72 – 0.76 | the scramble, the zero guard, two stores; the before (two stores) was not timed separately — the difference is `splitmix64`'s |
| `splitmix64(seed)` | 0.5 – 0.9 | add, `x ^ x >> 30`, multiply, `x ^ x >> 27`, multiply, `x ^ x >> 31` |
| one `next_u32` | 1.50 – 1.53 | unchanged code |

A seed is built once per seed change, so the change costs under a nanosecond per project start.

## The test suite

| build | before (baseline matrix) | after |
|---|---|---|
| `cargo test` (the `all` binary) | 0.67 s | 0.85 s — eight new tests; the play-level sweep is about 0.3 s of it |
| `cargo test --features mixer` | under 2 s | 2.04 s |

**A first form of the play-level test built an app per seed — 120 of them — and the mixer step took 13.21 s**, each app opening an audio device (about 90 ms) (`evidence/before/matrix-with-120-apps.txt`). It is now one reseeded app and 600 seeds.
