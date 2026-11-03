# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.0 / 3.8 / 4.0**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.025 | 0.0031 |
| 64 | 0.148 | 0.0023 |
| 256 | 0.490 | 0.0019 |

**Unchanged in the crate**: no `src/` line moved and `perfcheck` does not run the example. The 256-voice figure (0.490 µs)
sits inside the 0.4–1.4 µs spread this binary has shown over iterations 36–39 on a machine at the same load.

**What the change costs, where it runs.** `Kind::Bus` already walked `SirenLog::recent(CAPACITY)` to find the newest duck;
it now also compares that entry's `by_db` with `mix.ducked` — one float comparison per **ducked bus**, four at most.

## The probe (the real before/after)

`evidence/panel/`, plus a pixel scan of each still:

| | before | after |
|---|---|---|
| `music.stinger` ever played | **no** — `Stinger` never written | `0.84s stinger ducked Music by -6.0 dB` |
| the `Music` row with two ducks | `duck -8.0 dB **by stinger**` — wrong cause | `duck -8.0 dB **(newest: stinger at -6.0)**` |
| the `Effects` row with one duck | `duck -4.0 dB by dialogue.greeting` | **unchanged** |
| the bus bands | 4, evenly spaced | 4, evenly spaced (196, 227, 260, 292) — no wrap |
| the still's margin | 11 px | **11 px** |
| the GIF's first frame | 0.67 s, before the stinger | **0.94 s, on the stinger** |
