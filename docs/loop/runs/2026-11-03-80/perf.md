# Iteration 80 — perf

**Case 19, measured for the first time since iteration 75.** `cargo run --release --example perfcheck`, Apple M4,
`--release`, machine otherwise idle.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | **0.084** | 0.0105 |
| 64 | **0.472** | 0.0074 |
| 256 | **1.576** | 0.0062 |

`perfcheck`'s own footer: *"upper bound: every voice is costed, including the ones `update_mix` skips because their gain
did not move"* — so this is the crate's **per-frame** work, not the audio thread's, and it is an upper bound.

**No regression.** The change is one assertion in `tests/library.rs`; nothing in `src/` moved. Every figure above is the
same code iteration 75 measured, and the per-voice cost falls with count (0.0105 → 0.0062 µs) as it must when a fixed
per-frame cost is amortised.
