# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **2.42 / 4.40 / 5.62**; disk 14 GB free.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.018 | 0.0022 |
| 64 | 0.108 | 0.0017 |
| 256 | 0.537 | 0.0021 |

**No `src/` file changed this iteration**, so the per-frame work is the same code as iteration 10's. The figures are
lower than that run's (0.065 / 0.354 / 1.389) because the machine's load is 2.4 rather than 4.7 — the whole difference
is the machine, and presenting it as an improvement would be false.

## What *is* measured

The mutation, which is the evidence this iteration has:

```
steal's `!c.unstealable` filter removed ->
  mix::an_unstealable_voice_is_never_taken_by_a_stealable_request ... FAILED
  mix::the_table_budget_refuses_rather_than_grow ... FAILED
  mix::a_full_bus_names_that_bus ... FAILED
  (12 passed, 4 failed)

restored -> 16/16 mix tests green
```

and the five negative probes in `research.md`, each a measurement rather than an impression.
