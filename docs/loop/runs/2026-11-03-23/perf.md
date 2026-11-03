# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **10.2 / 13.8 / 17.5**.

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.336 | 1.035 |
| 2 | 0.384 | 1.391 |
| 3 | 0.355 | 1.403 |

**Median 256 ≈ 1.391 µs**, against iteration 22's 1.31 — 6 % apart, inside the 3.4× spread that binary showed last
iteration on this machine. **The change cannot be on this path**: `Rng::seed()` returns a `u64` field that is written
**once**, in `Rng::new`; `next_u32` is the same three xorshifts it was. `SirenChoices::seed()` is called by a panel or a
test, not per frame.

**What the change costs, exactly:** one extra `u64` in `Rng` (16 bytes instead of 8 for the struct) and one branch in
`Rng::new`. A voice stream is built once per `SirenChoices`, which is once per seed change.

## The probe (the real before/after)

`evidence/seed{,-before}.txt`, three settings, through `SirenPlugin`:

| settings | before: `seed()` | after |
|---|---|---|
| `seed: 0` | **`0`** — a number the stream did not come from | `6840213216811810817` |
| default seed | `6840213216811810817` | `6840213216811810817` (guard) |
| `seed: 1` | `1` | `1` (guard) |

Both before and after, `seed: 0` and the default produce the **same six sounds** — the behaviour is unchanged; only the
report now says so.
