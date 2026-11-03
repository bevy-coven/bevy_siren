# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.2 / 5.8 / 5.3**.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.057 | 0.062 | 0.0072–0.0078 |
| 64 | 0.327 | 0.358 | 0.0051–0.0056 |
| 256 | 1.241 | 1.157 | 0.0045–0.0048 |

**What the change costs.** `worst_peak_db` is one lookup of the voice count, one of the bus gain, and `sum_db` over `n`
values — `n` is a bus's live voices, at most `max_voices` (32 by default). **It is not called by any system**: only a
caller that asks, which is the panel and a game's own meter. `update_mix` is untouched.

**The probe** — `evidence/panel/headroom-40.png` and `evidence/after/render.txt`, read:

| | before | after |
|---|---|---|
| 307 samples past full scale in `voices-8.wav` | *"the budget's job, not the gain's"* — **false** | *"the budget is a sample cost, not headroom"* |
| a game asking "can my table clip" | no API existed | `SirenMix::worst_peak_db(bus)` |
| the shipped table, two voices on `Effects` | invisible | **`+6.0 dBFS / 2`** in the `Music` block |
| the shipped table, two voices on `Music` | invisible | **`+4.0 dBFS / 2`** |
| the `Problems` block | `none` | **`none`** — the table is not wrong, and the peak is not its question |
| tests | 127 / 134 / 138 + 5 | **128 / 136 / 139 + 6** (one test, one doctest) |
