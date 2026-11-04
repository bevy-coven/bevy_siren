# Iteration 110 — perf

Machine: Apple **M4 Pro**, 14 cores, 48 GB, macOS, shared with the owner's other work. Load average **1.9–2.3** during the
`perfcheck` runs (it read 14.8 in iteration 109's worst sample, so this was a quiet hour).

## The mix path (the skill's 8 / 64 / 256 voices)

This iteration changes **no code on the mix, music or play path** — the diff is `ModelStamp::clap`, a flag on a binary, an
example, a Python script, tests and documents — so the check is that nothing moved. `cargo run --release --example perfcheck`,
three runs, µs per frame (the crate's own per-frame work; an upper bound, because every voice is costed including the ones
`update_mix` skips):

| voices | run 1 | run 2 | run 3 | prior band (iterations 108–109) |
|---|---|---|---|---|
| 8 | 0.063 | 0.063 | 0.018 | 0.041–0.070 |
| 64 | 0.337 | 0.364 | 0.114 | 0.107–0.395 |
| 256 | 0.765 | 0.447 | 0.408 | 0.401–1.339 |

All nine samples are inside or below the band. The spread inside this run (0.018 to 0.063 at 8 voices) is the machine, not the
code: run 3 is the same binary as run 1.

`render`: **21 of 21 WAVs byte-identical** to iteration 109's (`cmp`), master **−21.5 LUFS, −7.5 dBFS true peak**, unchanged.

## What the iteration measured for the first time: the real graphs

These are the numbers the loop could not give before, because the graphs were not on this machine. Warm file cache; a cold
load is **not measured**.

| what | figure | how |
|---|---|---|
| audio graph load (117 MB) | 0.2–0.3 s | `examples/semantic`, the three runs that indexed (the earlier failing runs read 0.3–1.4 s, with the cache in an unknown state) |
| text graph load (501 MB) | 0.2–0.4 s | same |
| text encode, one description | **5.4–6.8 ms** | same; `siren-tools` `search_music` round trip **4.6–5.6 ms** |
| ranking 14 assets | 0.01–0.13 ms | same |
| **indexing, real tower** | **16.6 clips/s = 60 ms a clip** | 280 distinct ~1 s clips, 16.9 s, decode and loudness included |
| &nbsp;&nbsp;of which the tower | **35.1 ms** | scratch timing, 30 repetitions, one clip (matches ADR 0002's 34.9 ms) |
| &nbsp;&nbsp;of which the crate's mel spectrogram | **26.0 ms** | same; 4 channels, padded to the 10 s window |
| `siren-tools` start-up with the real text graph | 0.9 s | spawn to first reply: index read + graph load |

**Projection, from the measurement, not a target:** 5 000 clips at 60 ms is **301 s, about five minutes**, once; the decode
alone is 3 s. The tower is 58 % of a clip's cost and **this crate's own mel is 42 %** — and the two run one after the other, so
a library build is serial where it need not be.

**The mel stage is the finding.** 26 ms for one 10 s window is a 1001-frame, `f64` FFT plus a 64 × 513 filter — the precision
the reference promotes to, chosen on purpose in iteration 76. Whether it can be faster at the same 0.0545 dB agreement, or
overlapped with the tower on another thread, is a measurement away, and it is the cheapest cut to a five-minute index. Not
attempted here: it is a different item.

**GPU:** still not measured (`#36`). Nothing here has a GPU in it.
