# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **5.88 / 7.01 / 6.31**; disk 2.1 GB free, 100 % used.

**What this number is.** The crate's per-frame *game-thread* work — `model::chain_db` and `model::db_to_gain` per live
voice plus `model::approach` per bus — the shape of `update_mix`. An **upper bound**: the real `update_mix` skips a
voice whose gain moved less than 0.01 dB.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.022 | 0.0027 |
| 64 | 0.096 | 0.0015 |
| 256 | 0.667 | 0.0026 |

**The change is not on this path and cannot be.** `room_for` runs once per **request**, and only its *refusal* changed:
`Option<Vec<Entity>>` became `Result<Vec<Entity>, Cap>` — the same size, the same `Some`/`Ok` fast path, one enum
instead of `None`. A frame with no play request executes none of it. `perfcheck` drives neither `room_for` nor the
music systems, so a comparison against iteration 4's numbers (0.019 / 0.103 / 0.684 under load 2.7) is the machine's
load moving, not this change. What the harness confirms is that no per-voice path moved, which is the regression the
loop's rule cares about.

## What *is* measured

`examples/budget`'s new `refusals.txt`, from the real ECS with a real WAV per cue:

```
before:  table budget: VoiceLimit { bus: Some("Ambience") } -> no voice free on bus `Ambience`
         cue polyphony: VoiceLimit { bus: Some("Effects") }  -> no voice free on bus `Effects`
         bus voices:    VoiceLimit { bus: Some("Effects") }  -> no voice free on bus `Effects`

after:   table budget: VoiceLimit { bus: None }              -> no voice free
         cue polyphony: VoiceLimit { bus: None }             -> no voice free
         bus voices:    VoiceLimit { bus: Some("Effects") }  -> no voice free on bus `Effects`
```

Two shapes where there was one, and the only remaining bus name is the one that is true.
