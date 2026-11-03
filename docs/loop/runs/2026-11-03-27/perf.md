# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **16.9 / 8.6 / 8.5** — a spike in the first sample, the same pattern iteration 22 hit.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 (load 16.9) | 0.345 | 1.338 |
| 2 | 0.105 | 0.552 |
| 3 | 0.361 | 1.371 |

Median 256 ≈ **1.338 µs**, against iteration 26's 1.219 — 10 % apart on a machine whose load average swung by 2× between
runs. The spread within one binary here is **2.5×**, which is larger than any difference this change could make.

**The change's own cost, which is what matters:** `update_mix` gains one pass over the **duck rules** (two by default,
one per `Dialogue` target) per frame — `live.get(&rule.source)` and, when the source is quiet and the hold has not
elapsed, one subtraction. It is not per voice. `SirenMix::voices`, the panel and the per-voice gain loop are untouched.

## The probe (the real before/after)

`evidence/duck-hold{,-before}.txt`, `Music`'s duck every 10 frames, 60 fps, 70 frames:

| case | before | after |
|---|---|---|
| the line is **stopped** on frame 0 | `0:-8.0 10:-8.0 20:-6.9 … 60:+0.0` | `0:-8.0 10:-6.1 20:-3.9 … **40**:+0.0` |
| a **loop** keeps playing | `… 60:+0.0` | unchanged |
| a **second line** at frame 20 | extended past 70 | unchanged |
| a **one-shot that ends by itself** | `… 60:+0.0` | unchanged |
