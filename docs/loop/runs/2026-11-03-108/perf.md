# Iteration 108 — perf

Machine: this Mac, shared; `uptime` load average **3.71 / 3.04 / 3.36** at the measurement. Nothing here is on any hot path:
the change is to a test, and `src/` is byte-identical.

## Per-frame audio cost (control)

`cargo run --release --example perfcheck`, the crate's per-frame work as an upper bound (every voice is costed, including the
ones `update_mix` skips because their gain did not move):

| voices | this run | µs/voice | band across unchanged code |
|---|---|---|---|
| 8 | 0.063 | 0.0079 | 0.041–0.070 |
| 64 | 0.355 | 0.0055 | 0.107–0.395 |
| 256 | 1.218 | 0.0048 | 0.401–1.339 |

Inside the band iteration 102 recorded and iteration 107 repeated. The 256-voice figure sits in the upper half of a spread
that has already been shown to be the machine's rather than the code's (three runs earlier in this same iteration read 0.668,
1.222 and 0.997). **No speedup or slowdown is claimed.**

## What the change costs at run time

**Zero.** It touches `tests/table.rs` and docs. A test binary is not a consumer's binary, and `src/` did not change.

## What it saves

A table-level report that quietly omits a kind. `InertAvoidRepeat` was tested on its own (`tests/play.rs` asserts it fires,
and that legal windows and `Shuffle` do not trigger it; `tests/model.rs` proves the behaviour) but was absent from the one
test whose job is *every problem prints as a sentence naming its cue*. The new guard turns that omission into a failing test.
