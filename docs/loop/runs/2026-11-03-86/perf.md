# Iteration 86 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`.

| voices | µs/frame | µs/voice | iteration 85 |
|---|---|---|---|
| 8 | **0.059** | 0.0073 | 0.063 |
| 64 | **0.329** | 0.0051 | 0.365 |
| 256 | **1.141** | 0.0045 | 0.963 |

**No regression.** The change is inside `semantic`, which the default build does not compile, so `perfcheck`'s binary is
unchanged in the audio path. Across iterations 80–86 the same audio code reads 0.059–0.084 / 0.329–0.472 /
**0.501–1.576** µs — the 256-voice figure has ranged over **3×** on identical code.

## The cost this iteration adds, and where it is measured

`build` now does one `read_to_string` plus one parse of a CSV or JSON sidecar per folder, **once per build** — not per
file. On a library of any useful size that is noise beside the decode it schedules. The number that would show it is
`#36`'s indexing throughput, which has still not been taken and now has three added costs to account for: loudness
measurement, sidecar parsing and the path→hash join.
