# Iteration 112 — per-frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, three runs back to back on the shared Mac (load average 1.40 / 2.75 / 4.11 — a quiet machine, the
ESC-50 runs having finished). The figure is the crate's own per-frame work, as an upper bound: every voice is costed, including those `update_mix`
skips because their gain did not move. It is not the audio thread's cost.

| voices | run 1 (µs / frame) | run 2 | run 3 | recorded band, iterations 80–111 |
|---|---|---|---|---|
| 8 | 0.062 | 0.018 | 0.020 | 0.018 – 0.074 |
| 64 | 0.354 | 0.106 | 0.116 | 0.107 – 0.395 |
| 256 | 1.258 | 0.396 | 0.427 | 0.401 – 1.339 |

**No regression, and none expected.** The audio path is not touched: this iteration changed `Cargo.toml` (two more decoder features on `rodio`, which
`perfcheck` does not use), `library::read_header`, `semantic::decode` and an example. Run 1 is the cold run, as in earlier iterations; runs 2 and 3
sit at the bottom of the band, and run 2's 64- and 256-voice figures (0.106, 0.396) are a hair under it. No claim is made from that.

**The render is the stronger check.** All **21 of 21** rendered WAVs are byte-identical to iteration 111's (`cmp`), the master reads **−21.5 LUFS
integrated, −7.5 dBFS true peak** (`evidence/after/loudness-mix.txt`), and the spectrograms of the master mix and of the stone footstep were read
(`evidence/after/spectrograms/`): a broadband attack in the first 25 ms, then steady tonal lines to the end, no clicks, gaps, clipping edges or
aliasing images.
