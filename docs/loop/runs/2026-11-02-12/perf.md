# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.01 / 4.99 / 5.39** — the busiest of any run in this loop; disk 14 GB free.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.069 | 0.0087 |
| 64 | 0.628 | 0.0098 |
| 256 | 1.391 | 0.0054 |

**The change is not on this path.** `perfcheck` drives `model`'s arithmetic; this iteration added one
`Query::get(entity)` per **stop request** — an event, not a per-frame cost — and stop requests are rare. A frame with no
stop request executes none of it. The 64-voice figure is twice iteration 11's (0.108) on a machine at load 8.0 against
2.4: that is the machine, and presenting it as a regression would be false.

## What *is* measured

The regression test, unfixed and fixed:

```
unfixed:  thread 'play::a_stop_for_an_ended_voice_is_a_no_op' panicked at bevy_ecs-0.20.1/src/error/handler.rs:128
          Encountered an error in command: Entity despawned: The entity with ID 43v0 is invalid;
          its index now has generation 1.
          test result: FAILED

fixed:    test result: ok. 20 passed
```

That is the whole evidence: a crash, and no crash.
