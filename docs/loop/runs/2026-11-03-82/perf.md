# Iteration 82 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`, idle.

| voices | µs/frame | µs/voice | iteration 81 |
|---|---|---|---|
| 8 | **0.062** | 0.0078 | 0.064 |
| 64 | **0.342** | 0.0053 | 0.355 |
| 256 | **1.227** | 0.0048 | 1.341 |

**No regression, and no claim of an improvement.** The change is a new module behind the non-default `semantic` feature;
nothing in the audio path moved. Across iterations 80–82, identical audio code now reads 0.062–0.084 / 0.342–0.472 /
1.227–1.576 µs, which is the run-to-run spread on a shared machine and belongs in the table rather than in a conclusion.
