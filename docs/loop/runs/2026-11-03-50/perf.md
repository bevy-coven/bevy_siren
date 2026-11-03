# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.6 / 4.3 / 4.7** — the quietest, steadiest run of the last ten iterations.

| voices | run 1 µs/f | run 2 µs/f | µs/voice |
|---|---|---|---|
| 8 | 0.047 | 0.062 | 0.0059–0.0078 |
| 64 | 0.286 | 0.327 | 0.0045–0.0051 |
| 256 | **0.501** | **0.500** | 0.0020 |

**The two runs agree to a microsecond at 256 voices**, which is unusual for this host and is the strongest run the loop
has recorded. Nothing in `src/` changed this iteration: the work is in `examples/mixer`'s bus levels, `examples/render`'s
`mix.wav` gain and the guide.

**The probe** — `evidence/panel/trimmed-96.png`, `evidence/after/render.txt`, `ebur128`:

| | before | after |
|---|---|---|
| `examples/mixer`'s `Effects` | `+0.0 dB`, `peak +6.0 dBFS / 2` | **`−7.0 dB`**, no peak clause |
| `examples/mixer`'s `Music` | `−2.0 dB`, `peak +4.0 dBFS / 2` | **`−7.1 dB`**, no peak clause |
| the `Music` block's tail | a peak clause on both buses | **no clause** — nothing reaches 0 dBFS |
| `render`'s `mix.wav` true peak | **`+0.2 dBFS`** (4.2× full scale) | **`−6.1 dBFS`** |
| `render`'s `mix.wav` integrated | `−4.3 LUFS` | **`−18.6 LUFS`** — below the −14 a game mix targets |
| the `mix:` report line | `10 cues, 18.583311 dB summed` | `… (all ten at once through nothing; this file is written at \`master_db\` +0 dB)` |
| `worst_peak_db` vs `master_db` | untested | **1:1**, asserted at `0`, `−6`, `−20` dB |
| tests | 128 / 136 / 139 + 6 | **129 / 137 / 140 + 6** |
