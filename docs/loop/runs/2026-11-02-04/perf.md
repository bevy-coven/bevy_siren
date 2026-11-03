# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.68 / 3.23 / 4.42**; disk 2.1 GB free, 100 % used.

**What this number is.** The crate's per-frame *game-thread* work: `model::chain_db` and `model::db_to_gain` per live
voice plus `model::approach` per bus — the shape of `update_mix`. An **upper bound**: the real `update_mix` skips a
voice whose gain moved less than 0.01 dB.

| voices | before, µs/frame | after, µs/frame | before, µs/voice | after, µs/voice |
|---|---|---|---|---|
| 8 | 0.019 | 0.019 | 0.0024 | 0.0024 |
| 64 | 0.103 | 0.103 | 0.0016 | 0.0016 |
| 256 | 0.684 | 0.684 | 0.0027 | 0.0027 |

**Identical, and the reason is exact.** `perfcheck` drives `model`'s pure arithmetic, not the music systems. The change
adds one `f32::ln` per layer per frame — a music score has two to six layers, and `1000f32.ln()` is computed on a
constant, so an optimising build folds it — plus one `dt <= 0.0` branch. At 256 voices this is 0.684 µs of a 16.7 ms
frame, 0.004 %, and the music path's own cost is six `ln` calls at most, which is below the harness's resolution.

The honest statement: **this harness cannot see the change.** What it does confirm is that no per-voice path moved,
which is the regression the loop's rule cares about.

## What *is* measured

`examples/render`'s new `layer-fade` line and the plot it writes, both from `model::tau_for_fade` and `approach`:

```
layer-fade: from -80 to -9 dB; 0.3 s in, the authored 0.8 s fade reads -13.6 dB and the default easing reads -9.1 dB
```

and `tests/model.rs::a_fade_arrives_within_one_part_in_a_thousand` sweeps four durations at 240 fps. Those are the
numbers this change is about; frame cost is not one of them.
