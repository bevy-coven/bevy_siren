# Iteration 95 — eval

## Baseline, before anything

| check | result |
|---|---|
| `cargo build` | green |
| `cargo test` | 198 passed |
| `cargo test --features serde` | green |
| `cargo test --features mixer` | green |
| `cargo test --doc` | 6 passed |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `cargo build --examples` | green |

**Nothing was red.** The item came from `log.md`'s own top-5 and from iteration 93's measured regression, not from a
failing case.

## The cases

`cues.yaml`'s 33 cases are unchanged and all green; this iteration touches no cue path. The observable is the indexer:

| Case | Green/red | Observation |
|---|---|---|
| decode throughput | red before, **green now** | 419.8 / 568.8 clips/s → 698.6 / 784.0 across three runs |
| spectral, 10 tests | **green, unedited** | the same numbers from a shared pass |
| tempo, 13 tests | **green, unedited** | including the 80-tempo corpus |
| analysis, 5 tests | **green, new** | frame count, tone bin, silence, onset sign, determinism |
| loudness / key | **green, untouched** | same modules, same call shape |

## The item

`src/semantic/analysis.rs` — one FFT per frame, consumed by `spectral` and `tempo`. Plus: `tempo`'s per-frame allocation
of two 16 KB vectors, removed by hoisting them out of the loop.
