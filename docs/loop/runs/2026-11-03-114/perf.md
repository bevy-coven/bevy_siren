# Iteration 114 — the gate: render, spectrograms, loudness, cost

The change is in the **`mixer` backend**, which `examples/render` does not run (it renders on Bevy's path), so the standard render is the *control*: it must not move. The change's own evidence is a second program that does run the mixer's chain.

## Render (the control)

`cargo run --release --example render -- docs/loop/runs/2026-11-03-114/evidence/after` — **21 of 21 WAVs byte-identical to iteration 113's** (`cmp` over every one). `mix.wav`: **−21.5 LUFS integrated, −7.5 dBFS true peak**, as before.
`spectrograms/mix-spectrum.png` **read**: a broadband onset in the first 25 ms, then three steady partials to 0.50 s; no gap, no click, nothing at the Nyquist edge — the picture of 113's.

## The change's evidence: the mixer's chain, rendered both ways (`evidence/after/loopgain.txt`, source `loopgain.rs.txt`)

The real `rodio::Decoder` (on `tests/fixtures/tone.flac` and a 660 Hz WAV), `looped` as the crate had it and as it is, a real `rodio::Player` on a bus `rodio::mixer::mixer`, pulled offline while the gain is moved every millisecond. **Every output is
compared sample by sample with what the schedule commands** (`gain[n] × the sample at n mod one pass`):

| | before | after |
|---|---|---|
| A. one stem, gain 1.0 / 0.25 / fade to 0 — rms per pass (commanded 0.08837, 0.02209, 0.05109) | 0.08837, 0.08837, 0.08837 | **0.08837, 0.02209, 0.05109** |
| A. worst \|heard − commanded\| per pass | 0.000000, **0.093727, 0.124720** | **0.000000, 0.000000, 0.000000** |
| B. two stems — combat (660 Hz) per pass, dBFS (commanded −∞, −18.99, −18.06, −18.06) | **−∞, −∞, −∞, −∞** | **−∞, −18.99, −18.06, −18.06** |
| B. calm (440 Hz) per pass, dBFS (commanded −18.06 × 3, −27.81) | −18.06 × 4 | **−18.06 × 3, −27.81** |
| B. worst \|heard − commanded\| per pass | 0, **0.125000, 0.125000, 0.209198** | **0, 0, 0, 0** |
| C. samples a gain store is late — stores at 1, 100, 1 000, 10 000, 20 000, 32 000, 32 767, 33 000, 70 000 into the first pass of a 100 000-sample loop | **32 767, 32 668, 31 768, 22 768, 12 768, 768, 1, 32 536, 28 304** (the distance to the next 32 768-sample span) | **0** at every store |

**Loudness** (`evidence/after/loudness.txt`): `score-after.wav` −19.7 LUFS, −12.5 dBFS true peak (two 0.125-peak stems summing); `score-before.wav` −21.8 LUFS, −18.1 dBFS (calm only: combat never sounded). `loop-gain-after.wav` −25.3 LUFS because pass 2 is 12 dB down and pass 3 fades to nothing.
All four are longer than the 0.4 s gating block, so the figures are measurements and not the −70 LUFS an under-block clip reports.

**Pictures, read** (`evidence/spectrograms/`):
- `loop-gain-before-wave.png` — **one flat band for all three passes**; `loop-gain-after-wave.png` — full height, **a quarter of it**, then **a straight taper to zero**: the schedule, drawn.
- `score-before-spectrum.png` — one band at 440 Hz for two seconds, and **no 660 Hz line at any time**; `score-after-spectrum.png` — the 440 Hz band throughout until 1.5 – 1.6 s, where it drops, and the 660 Hz band **entering at 0.5 s**, the first bar of pass 2.
  Faint patches near 1.3 – 1.8 kHz at both ramps, about −70 dB, are the 1 ms staircase of the schedule: the output equals the commanded signal to 0.000000, so the fix did not make them.
- (`showwavespic` with `colors=white` on a transparent PNG is invisible to `read`; the skill's command needs `colors=blue` or a background. Noted for the skill.)

## Cost

**The loop chain per sample, steady state** (`loopgain.txt` §D, release build, the first pass cached, then 20 × 10 s pulled, best of 20, three runs each; a shared Mac, load average 1.1 / 2.1 / 4.0 before):

| run | before (gain inside the repeat) | after (repeat, then gain) |
|---|---|---|
| 1 | 3.243 ns | 3.227 ns |
| 2 | 3.233 ns | 3.226 ns |
| 3 | 3.215 ns | 3.214 ns |

**No measurable difference** (−0.5 %, −0.2 %, 0.0 %); **P3 holds** (±10 %). An earlier single run read +4.9 % (3.307 vs 3.469 ns) and was noise. In steady state the old chain *could* have been cheaper — it replayed cached, already-multiplied samples and read the atomic only on the first pass — and
the measurements show it was not. At 96 000 samples a second (48 kHz stereo) either is about **0.3 ms of CPU per second per looping voice**.

**`update_mix`'s per-frame cost** (`cargo run --release --example perfcheck`, three runs, µs per frame; the mixer feature is off in `perfcheck`, so this is a control, not the change):

| voices | run 1 | run 2 | run 3 | band of earlier iterations |
|---|---|---|---|---|
| 8 | 0.064 | 0.057 | 0.057 | 0.018 – 0.074 |
| 64 | 0.344 | 0.323 | 0.322 | 0.106 – 0.395 |
| 256 | 0.797 | 1.076 | 1.101 | 0.396 – 1.339 |

All nine figures are inside the band. `perfcheck` costs every voice, including those `update_mix` skips because their gain did not move, so it is an upper bound.
