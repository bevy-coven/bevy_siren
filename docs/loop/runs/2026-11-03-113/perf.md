# Iteration 113 — the gate: render, spectrogram, loudness, perfcheck

This iteration changes **no audio path**: the code is `src/semantic/eval.rs` (two pure functions and a struct), two doc comments (`rank.rs`, `weights.rs`), `examples/goldenbench.rs`
and `tests/semantic_golden.rs`. The gate is run anyway, because "the iteration cannot have changed the sound" is a claim and a render is how it is checked.

## Render

`cargo run --release --example render -- docs/loop/runs/2026-11-03-113/evidence/after` wrote 41 files. **21 of 21 WAVs are byte-identical to iteration 112's**
(`cmp` over every `*.wav` in `docs/loop/runs/2026-11-03-112/evidence/after/`).

## Loudness (`evidence/loudness-mix.txt`)

`mix.wav` (0.50 s, ten cues at once, written at `master_db` +0 dB): **−21.5 LUFS integrated, −7.5 dBFS true peak** — the figures of iterations 111 and 112. A 0.5 s file is longer than the
0.4 s gating block, so the figure is a measurement and not the −70 LUFS an under-block clip reports.

## Spectrograms, read (`evidence/spectrograms/`)

- `mix-spectrum.png` — a broadband onset in the first ~25 ms (the stinger and the footstep attacks), then three steady partials at roughly 150, 370 and 620 Hz (read off the axis, 40 px per 1 470 Hz)
  to the end of the file at 0.50 s; no gap, no click after the onset, no energy above about 1.5 kHz after 30 ms, nothing at the Nyquist edge.
- `footstep-grass-spectrum.png` — a 25 ms noise burst to 11 kHz, then three decaying partials at roughly 370, 1 470 and 2 200 Hz with a faint harmonic comb above them, falling to the floor by
  0.33 s; no clipping signature (no vertical broadband line after the attack), no truncated tail.

## Perfcheck (`cargo run --release --example perfcheck`, three runs, µs per frame)

Machine state: load average 2.37 / 2.51 / 2.60 before and after — a shared Mac (the CLAP benchmarks of this iteration ran on it), so the figures are an upper bound and compared against the bands of
earlier iterations rather than against a quiet machine.

| voices | run 1 | run 2 | run 3 | band of earlier iterations |
|---|---|---|---|---|
| 8 | 0.064 | 0.065 | 0.064 | 0.018 – 0.074 |
| 64 | 0.296 | 0.343 | 0.330 | 0.106 – 0.395 |
| 256 | 1.061 | 1.217 | 1.044 | 0.396 – 1.339 |

All nine figures are inside the band. As an upper bound, every voice is costed including the ones `update_mix` skips because their gain did not move, per the example's own line.
