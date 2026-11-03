# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.3 / 3.5 / 4.6** — quiet.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.065 | 0.0081 |
| 64 | 0.353 | 0.0055 |
| 256 | 1.056 | 0.0041 |

**Unchanged.** This iteration changes one literal in `examples/mixer.rs` — `recent(10)` to `recent(8)` — and nothing in
`src/`. The crate's per-frame work is identical in source; the 256-voice figure sits inside the 0.62–1.34 µs spread this
binary has shown across iterations 27–31.

**What the change does cost:** the panel formats **two fewer** log lines per frame. Strictly less work, unmeasurably small
against 8.3 ms.

## The probe (the real before/after)

A pixel measurement of the two captures — the only instrument that can see a clipped glyph, since `recent(10)` returns ten
entries whether the window shows them or not:

| capture | lowest ink | margin | log's last line |
|---|---|---|---|
| **before** (`readme-before.png`) | **y = 719** of 720 | **1 px** | **clipped mid-glyph** |
| **after** (`docs/media/mixer.png`) | **y = 708** | **12 px** | complete, eighth line |

And the free height this leaves, for the next item: **12 px**, against ~134 px for a four-line Music block.
