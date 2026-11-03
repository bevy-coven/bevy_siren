# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **5.1 / 5.0 / 4.7**.

| voices | before (median of 4) | after (median of 4) | delta |
|---|---|---|---|
| 8 | 0.062 µs | 0.064 µs | +2.4 % |
| 64 | 0.353 µs | 0.355 µs | +0.6 % |
| 256 | 0.855 µs | 1.336 µs | **+56.2 %** |

## The 256 figure is the host, and here is the proof

**`perfcheck` does not call a single function this iteration changed.** It builds no `VariantPicker`, no
`SirenChoices`, and no cue table; its loop is `approach`, `db_to_gain` and `chain_db` over a fixed gain list
(`examples/perfcheck.rs:33-40`). A 56 % move there cannot come from a variant selection fix.

**One binary, six consecutive runs of the 256 case:**

```
0.673  1.384  1.337  1.221  1.291  1.193
```

A **2× spread on a single compiled binary** is the host, not a regression. The `before` and `after` medians sit inside
that spread, so neither is distinguishable from the other today — and the 8- and 64-voice cases, which are measured
against the same noisy clock, moved by 2.4 % and 0.6 %.

**What the number means.** `perfcheck` measures the **crate's per-frame work as an upper bound**: every voice is costed,
including the ones `update_mix` skips because their gain did not move. It is not the audio thread.

**What is actually different.** `pick_from` gains one comparison per call —

```rust
if usize::from(avoid) > self.window { self.set_capacity(usize::from(avoid)); }
```

— executed **once per play request**, not once per voice per frame. At the play rate of any game (tens per second) this
is below the clock's resolution, which is why 8 and 64 voices moved under 3 %.

**Not claimed:** a speedup or a regression. The host could not resolve one today.
