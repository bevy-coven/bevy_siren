# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.09 / 5.50 / 6.20**; disk 828 MB free, 100 % used.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.063 | 0.0079 |
| 64 | 0.367 | 0.0057 |
| 256 | 1.347 | 0.0053 |

**The change is not on this path.** `perfcheck` drives `model`'s pure arithmetic; this iteration added a frame-local
`Vec<PendingStinger>` and one system that scans it. A frame with no stinger request pushes nothing and the new system
drains two message buffers and returns. At 256 voices the whole figure is 1.347 µs of a 16.7 ms frame (0.008 %), and
the load moved from 4.8 to 4.1 between runs — the difference between iterations 8 and 9 is the machine.

## What *is* measured

The probe, three lines, before and after:

```
before:  Music ducked:  -9.00 dB   refusals: [MissingSound { sound: "stinger_01" }]
         log:  0.03s stinger ducked Music by -9.0 dB
         log:  0.03s refused `music.stinger-no-handle`: no handle for sound `stinger_01`

after:   Music ducked:   0.00 dB   refusals: [MissingSound { sound: "stinger_01" }]
         log:  0.03s refused `music.stinger-no-handle`: no handle for sound `stinger_01`
```

and the same pair for a cue not in the table (`UnknownCue`, no duck). The positive case is asserted beside them:
a real stinger still reads −9.0 dB on the frame it fires.
