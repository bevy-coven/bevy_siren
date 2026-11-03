# perf.md — what was measured, and what was not

## The one real number

**CLAP's audio tower, 34.9 ms per clip, batch 1, on an Apple M4 CPU** (`CPUExecutionProvider`, no CUDA, no `fp16`).

That is the *model's* cost, not this crate's per-frame cost, and it is not comparable with `examples/perfcheck`'s
figure. The distinction matters: `perfcheck` measures `update_mix`'s work per frame as an upper bound, while this is one
offline inference call.

It is recorded because it bounds what `P-01` has to beat: 27.5 M parameters is small enough that call overhead, not
arithmetic, may dominate on a 3090.

## Why `perfcheck` was not run

No crate code on the audio path changed. Everything this iteration added is behind `semantic`, off by default, and
`cargo tree --edges normal` is **413 lines either way** — so a per-frame figure would be measuring the same code as
iteration 61 and reporting it as new.

**Recorded as a gap rather than skipped:** the next iteration that changes `src/mix.rs`, `src/play.rs` or
`src/playback.rs` owes a before/after at 8, 64 and 256 voices.

## Machine state

| | |
|---|---|
| local | Apple arm64, `bevy_siren/target` 19 GB, **7.8 GB free** |
| `bmb` | Apple M4, 48 GB RAM, **8.7 GB free** |
| `big` | Linux x86_64, RTX 3090, reachable |

Disk is the binding constraint on both hosts and is why the spike downloaded only CLAP's export and deleted nothing it
did not create.
