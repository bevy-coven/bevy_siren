# Iteration 85 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`.

| voices | µs/frame | µs/voice | iteration 84 |
|---|---|---|---|
| 8 | **0.063** | 0.0078 | 0.062 |
| 64 | **0.365** | 0.0057 | 0.352 |
| 256 | **0.963** | 0.0038 | 0.501 |

**No regression, and no claim to measure one.** The change is inside `semantic`, which the default build does not
compile, so `perfcheck`'s binary is byte-identical in the audio path. Across iterations 80–85 the same audio code reads
0.062–0.084 / 0.342–0.472 / **0.501–1.576** µs — the 256-voice figure has now ranged over **3×** on unchanged code,
which is the machine, not the crate.

## The one real cost this iteration adds, and it is not measured here

A build now runs one IIR pass per file over a buffer it already holds. That is **scan-side** work, not per-frame work,
and `perfcheck` does not exercise it — `#36` (indexing throughput) is the issue that owns it, and it now has a real
build to measure.
