# Iteration 84 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`.

| voices | µs/frame | µs/voice | iteration 83 |
|---|---|---|---|
| 8 | **0.062** | 0.0078 | 0.065 |
| 64 | **0.352** | 0.0055 | 0.358 |
| 256 | **0.501** | 0.0020 | 1.038 |

**No regression.** The change is inside `semantic`, which the default build does not compile. Across iterations 80–84
identical audio code reads 0.062–0.084 / 0.342–0.472 / **0.501–1.576** µs. The 256-voice figure's low end moved a lot
(0.501 against 1.038–1.576 in every prior run), which is the clearest evidence yet that this machine's variance is
larger than the effect any of these iterations could have — and is why the table shows a range rather than a trend.
