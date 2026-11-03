# Iteration 87 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`.

| voices | µs/frame | µs/voice | iteration 86 |
|---|---|---|---|
| 8 | **0.062** | 0.0078 | 0.059 |
| 64 | **0.354** | 0.0055 | 0.329 |
| 256 | **1.312** | 0.0051 | 1.141 |

**No regression.** The change is inside `semantic`, which the default build does not compile. Across iterations 80–87 the
same audio code reads 0.059–0.084 / 0.329–0.472 / **0.501–1.576** µs.

## The cost this iteration moves, in the right direction

`write` now emits **10.28 MB** where it emitted **31.88 MB** for 5 000 clips — a 3.1× smaller file, asserted by
`five_thousand_vectors_weigh_what_adr_0004_says` rather than quoted. Reading is a single `read` plus a bounds-checked
parse rather than a `read_to_string` plus float parsing per field, so load is cheaper too; no load-time number is
recorded because none was taken before this iteration, and inventing a "before" would be worse than saying so.
