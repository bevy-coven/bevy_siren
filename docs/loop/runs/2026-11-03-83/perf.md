# Iteration 83 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`.

| voices | µs/frame | µs/voice | iteration 82 |
|---|---|---|---|
| 8 | **0.065** | 0.0081 | 0.062 |
| 64 | **0.358** | 0.0056 | 0.342 |
| 256 | **1.038** | 0.0041 | 1.227 |

**No regression.** The change is inside `semantic`, which the default build does not compile. Across iterations 80–83
identical audio code reads 0.062–0.084 / 0.342–0.472 / 1.038–1.576 µs — the shared machine's spread, reported as a
spread rather than turned into a trend.

## The one new cost, and it is a type, not a loop

`Match` lost `Copy`. Where a caller previously moved a 12-byte value it now moves or borrows a struct holding two
`Option<String>`s. That is a compile-time cost at the call site and **no per-frame cost**: `top_k` was already cloning
into a `Vec<Match>`, and the loop over vectors is unchanged.
