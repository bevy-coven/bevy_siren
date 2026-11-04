# Iteration 119 — perf

What changed in a hot path: a new system, `owner::end_and_follow`, runs every frame directly after `play_requests`; `play_requests` asks `Entities::get_spawned` once per play that has an owner and takes `&Entities` and a tuple parameter instead of two; `mix::log_despawned_voices` (a system that ran every frame after `Mix`, with a `Local<BTreeMap>` it rebuilt each frame) is **gone**. Nothing in `update_mix` or the mixer's audio thread changed.

## The new system (`evidence/after/owner-system-cost.txt`)

A temporary `#[ignore]` unit test in `src/owner.rs` (removed before the commit): a bare `World`, one `Schedule` with Bevy's **single-threaded** executor (the default multi-threaded one reads 7 – 8 µs of setup at zero voices — iteration 117's lesson), only this system, 200 warm-ups and 20 000 timed runs, `cargo test --release --lib`, three runs on a shared machine.

| voices | run 1 | run 2 | run 3 |
|---|---:|---:|---:|
| 256, **none owned** (the archetype filter skips them) | 0.058 µs | 0.047 µs | 0.044 µs |
| 8 owned, placed | 0.189 | 0.110 | 0.099 |
| 64 owned, placed | 0.509 | 0.425 | 0.446 |
| **256 owned, placed** (an `Entities` lookup, a `GlobalTransform` lookup and an affine transform each) | **1.507** | **1.627** | **1.641** |
| 256 owned, not placed | 1.139 | 1.163 | 1.195 |

About 6 ns a voice with a follow, 4.5 ns without; **0.05 µs a frame for a game that owns nothing**. Prediction P8 (≤ 2 µs at 256 owned, placed voices; nothing measurable with none) held. The removed `log_despawned_voices` walked **every** voice every frame and rebuilt a `BTreeMap` of them; its cost was not measured and is gone, so the game that owns nothing pays *less* than before, not more — **[INFERENCE]** from reading the code, not timed.

## `perfcheck` (release, three runs; `update_mix`, **not touched by this change**; `evidence/after/perfcheck.txt`)

| run | 8 voices | 64 voices | 256 voices |
|---|---:|---:|---:|
| 1 | 0.062 µs | 0.359 µs | 1.225 µs |
| 2 | 0.057 µs | 0.322 µs | 1.169 µs |
| 3 | 0.057 µs | 0.304 µs | 1.053 µs |

Inside iteration 118's band (0.062 / 0.371 – 0.443 / 1.174 – 1.288 µs, and 117's 0.060 / 0.32 – 0.36 / 0.57 – 1.2): no change that this instrument can see, which is what it should read for a system it does not call.

## At the play

`play_requests` gained one `Entities::get_spawned` per play **that has an owner** and nothing for a play with none; it is bounded by the 4.5 ns a voice that the *256 owned, not placed* row above measures, which includes one such lookup and the loop around it. A play on a gone owner now skips the steal pass, the live list and the duck check — less work, on a path that is rare.
