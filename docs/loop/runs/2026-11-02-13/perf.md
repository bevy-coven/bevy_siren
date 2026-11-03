# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.76 / 4.52 / 5.30**; disk 12 GB free.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.062 | 0.0078 |
| 64 | 0.361 | 0.0056 |
| 256 | 1.262 | 0.0049 |

**No `src/` behaviour changed** — this iteration edited two doc comments, one guide section and added two tests. The
figures are iteration 12's code at a lower load (2.8 against 8.0), and the difference is the machine. `perfcheck` drives
`model`'s arithmetic, so it could not see a doc change in any case.

## What *is* measured

The probe, and the mutation that shows the new test pins the documented reading:

```
probe:   two voices at max_instances: 2, table edited to 1, third play
         -> refused VoiceLimit, 2 voices still live, nothing stopped

mutation: remove room_for's unstealable guard (the shape a "reconcile by ending" implementation has)
         -> mix::a_cap_lowered_under_live_voices_does_not_end_them ... FAILED
```
