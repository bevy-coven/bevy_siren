# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.49 / 2.62 / 5.05** (after run) and **3.30 / 8.02 / 14.33** (earlier run the same
hour); disk 2.1 GB free, 100 % used. The machine is shared and its load moved by a factor of four between the two
runs below, so the comparison is indicative rather than tight.

**What this number is.** The crate's per-frame *game-thread* work: `model::chain_db` and `model::db_to_gain` per live
voice plus `model::approach` per bus — the shape of `update_mix`. It is an **upper bound**, because the real
`update_mix` skips a voice whose gain moved less than 0.01 dB.

| voices | before, µs/frame | after, µs/frame | before, µs/voice | after, µs/voice |
|---|---|---|---|---|
| 8 | 0.075 | 0.023 | 0.0094 | 0.0029 |
| 64 | 0.326 | 0.135 | 0.0051 | 0.0021 |
| 256 | 1.090 | 0.508 | 0.0043 | 0.0020 |

**The change is outside this harness entirely and the numbers say so.** `perfcheck` drives `model`'s pure functions;
this iteration touched `play::seed_choices` (one `u64` comparison per frame), one `DuckClock::manual` argument, and
`retire_voices` not at all. Nothing on the per-voice path moved. The ~2× difference above is the machine: the "before"
run was taken at load 8–14 and the "after" run at load 2.5, on the same code that `perfcheck` exercises.

The added work per frame is exactly one comparison:

```rust
if choices.seeded == settings.seed { return; }
```

A `u64` read from a resource, one compare, and a branch taken on every frame of a normal run. At 256 voices the whole
mix is 0.51 µs of a 16.7 ms frame — 0.003 % — and the seed check is a fraction of a nanosecond of that. No case moved
by 10 % in the direction the change could cause.
