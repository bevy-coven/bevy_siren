# perf.md — the model's numbers, and why the crate's were not re-measured

## Measured this iteration

| Figure | Value | What it is |
|---|---|---|
| text tower, batch 1, **M4 CPU** | **5.28 ms per query** | one encode |
| text tower, batch 4, M4 CPU | **7.34 ms per call** | 1.8 ms per item at batch 4 |
| `ort` build, macOS arm64 | **3.2 s** without the runtime, **8 s** with | build cost |
| `rubato` build | **2.12 s** | build cost |

**5.28 ms is the number that decides whether a query feels instant.** A panel that re-queries on a keystroke has
~16 ms before it drops a frame, so a query fits with room for the index scan beside it.

## Why `examples/perfcheck` was not run

No crate code on the audio path changed. Every addition this iteration is behind `semantic`, off by default and proven
so: `cargo tree --edges normal` is **413 lines** with no features and **424** with `--features semantic`, and `ort`
appears in exactly one of them. A per-frame figure would measure iteration 62's code and report it as new.

**Recorded as a gap:** the next iteration that changes `src/mix.rs`, `src/play.rs` or `src/playback.rs` owes a
before/after at 8, 64 and 256 voices.

## Machine state, because a number without it means nothing

| | |
|---|---|
| local | Apple arm64, `bevy_siren/target` 22 GB, **1.2 GB free at the end of the run** |
| `bmb` | Apple M4, 48 GB RAM — where the model work ran, so the local disk did not fill with weights |
| `big` | RTX 3090, reachable, **not used** — the CUDA path is untested |

**Disk is now the binding constraint on this machine.** 1.2 GB free is below what a release build needs (~2 GB), so
`cargo run --release --example perfcheck` would fail as `ld: write() failed, errno=28` — the failure the skill's own
notes warn looks like a code error and is not one. Nothing was deleted: `target` is 22 GB of build artifacts this
session did not create.
