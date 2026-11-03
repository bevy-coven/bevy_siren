# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **6.18 / 6.47 / 6.68**; disk 2.0 GB free, 100 % used.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.059 | 0.0074 |
| 64 | 0.329 | 0.0051 |
| 256 | 0.827 | 0.0032 |

**What this number is.** The crate's per-frame *game-thread* work — `model::chain_db`/`db_to_gain` per live voice and
`model::approach` per bus — the shape of `update_mix`. An upper bound, because the real system skips a voice whose gain
moved less than 0.01 dB.

**The change is not on this path.** `perfcheck` drives the pure arithmetic; this iteration touched `start_pending` and
one `Display` string. The added work is one `CueTable` lookup **per waiting voice per frame** — a voice is waiting only
while its bytes load, normally a frame or two — plus the table's existing `Res` read. At 256 voices the whole
`perfcheck` figure is 0.827 µs of a 16.7 ms frame (0.005 %), and the waiting lookup is not in that figure at all.

The numbers above are *higher* than iteration 5's (0.022 / 0.096 / 0.667) because the machine's load is 6.2 rather than
2.7; the same binary shape, a busier machine. What the harness confirms is that no per-voice path moved.

## What *is* measured

`examples/budget`'s `missing-sound.txt`, from the real ECS:

```
before:  no handle:     MissingSound { sound: "step_03" }        -> … for sound `step_03`
         undecodable:   MissingSound { sound: "footstep.grass" } -> … for sound `footstep.grass`

after:   no handle:     MissingSound { sound: "step_03" }        -> … for sound `step_03`
         undecodable:   MissingSound { sound: "step_03" }        -> … for sound `step_03`
```

The before tree named two different subjects for two causes of one variant; the after tree names the key both times.
