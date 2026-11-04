# Iteration 111 — perf

Machine: Apple **M4 Pro**, 14 cores, 48 GB, macOS, shared with the owner's other work. **Load average 5.7 – 6.1** (10.5 over five
minutes) during `perfcheck`, because the test matrix had just finished compiling — a busier hour than iteration 110's 1.9 – 2.3, so the
numbers carry the noise and no claim is made from them.

## The mix path (the skill's 8 / 64 / 256 voices)

This iteration changes **no code on the mix, music or play path**: the diff is a new `semantic` module, additions to `semantic::eval`,
docs, tests, an example, a Python script and data. So the check is that nothing moved. `cargo run --release --example perfcheck`, three
runs, µs per frame (the crate's own per-frame work; an upper bound, because every voice is costed, including the ones `update_mix`
skips because their gain did not move):

| voices | run 1 | run 2 | run 3 | prior band (iterations 108 – 110) |
|---|---|---|---|---|
| 8 | 0.059 | 0.074 | 0.063 | 0.018 – 0.070 |
| 64 | 0.308 | 0.359 | 0.358 | 0.107 – 0.395 |
| 256 | 0.436 | 1.187 | 1.320 | 0.401 – 1.339 |

Eight of nine samples are inside the band. The ninth — **0.074 at 8 voices, 6 % above the band's 0.070** — is run 2 under a load of 6;
runs 1 and 3 of the same binary read 0.059 and 0.063. The spread inside this run (0.436 to 1.320 at 256 voices) is the machine. No
claim; the rendered audio is the evidence nothing moved.

`render`: **21 of 21 WAVs byte-identical** to iteration 110's (`cmp`), master **−21.5 LUFS, −7.5 dBFS true peak**, unchanged.

## What the iteration measured for the first time: the real model on labelled recordings

| what | figure | how |
|---|---|---|
| fetch the golden set, cold | **79 s** for 2 000 files / 880 MB (11 MB/s) | `tools/fetch_esc50.py`, 16 workers, `raw.githubusercontent.com` |
| the same through GitHub's archive | **~100 KB/s**: 193 MB in 30 minutes before it was killed, so ~2 hours for the whole | `urllib` and `curl` both, measured twice |
| verify a complete set | 1.0 s the first time, 0.5 s after | SHA-256 of 2 000 files |
| **index 2 000 clips, real tower** | **211 s = 9.5 clips/s = 105 ms a clip** | 5 s, 44.1 kHz mono clips; decode, resample, loudness, spectral, tempo and key included; **load average 6.5 at the start** |
| &nbsp;&nbsp;iteration 110, for comparison | 60 ms a clip | ~1 s, 48 kHz clips, no resampling; the mel stage padded to 10 s either way |
| text tower: 66 descriptions + 50 prompts | 0.9 – 1.1 s (5.4 s while a build was running) | including tokenisation; the graph loads in 0.2 – 0.4 s |
| rank 66 queries over 2 000 clips | 65.7 – 87 ms (128 ms while a build was running) | `rank::top_k` with `k = 2000`, ~1 ms a query |
| the independent `transformers` reference over all 2 000 clips | 145 s | `evidence/after/golden_parity.py` |

**What the 105 ms says.** A clip's cost is not constant: against iteration 110's 60 ms for one-second clips, five-second 44.1 kHz
clips cost **45 ms more**. Where those 45 ms go — the resampler, loudness (ebur128 over 5 s), the spectral pass, tempo, key — was not
decomposed here, and the load average makes the figure an upper bound. A 5 000-clip library of ESC-50-like clips is therefore about **9
minutes** on this CPU, not iteration 110's five. A quiet-machine figure was not taken.

**What it does not say:** nothing here has a GPU in it; a cold-cache model load is still unmeasured.
