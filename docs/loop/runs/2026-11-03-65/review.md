# review.md — iteration 65, three passes

## Pass 1 — Function: does the conversion do what the ratio says?

| Claim | Test | Result |
|---|---|---|
| 1.000 s at 44.1 kHz is **exactly 48000** at 48 kHz | `one_second_at_44_1k_becomes_exactly_48000_samples` | **48000** |
| a 1 kHz tone stays 1 kHz | `the_tone_keeps_its_frequency_through_the_conversion` | 998–1002 crossings |
| a 1 kHz tone keeps its energy | `the_tone_keeps_its_energy_within_the_measured_band` | 0.95–1.02 of 0.9763 |
| nothing folds to an image | `a_high_tone_survives_rather_than_aliasing` | 15 kHz survives, 9.1 kHz does not |
| awkward lengths keep their duration | `a_length_that_is_not_a_multiple_of_the_chunk_keeps_its_duration` | 1, 7, 1000, 1023, 1025, 4411 all exact |
| a 44.1 kHz file reaches the model | `a_file_at_another_rate_is_converted_not_refused` | 0.5 s in, 24000 out, tone intact |

**The cue cases are unchanged** — the render harness produced all 32 files and `render.txt` reports the same variant
sequences and gains as before (`footstep-grass` longest run **1**, `gun-burst` `avoid 0` with **12**, as authored).
Case 31 is still red and still blocked; it is not on this path.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the plots

**The bang's spectrogram** (`cue-bang-0.wav`, 0–0.31 s, 900×300): broadband energy at the attack decaying into a
low-frequency tail. **No vertical click line** at the start, **no flat clipping shelf** at the top of the colour scale,
**no gap** in the middle, and the tail decays to the noise floor rather than stopping abruptly.

**The mix waveform** (`mix.wav`, 900×200): a clean attack, a sustained body with the regular amplitude cycles a duck
produces, and a smooth decay to the tail. **No step** where the duck fires or recovers — a step in a gain curve is a
click, and there is none.

**Loudness** (`mix.wav`, `ebur128=peak=true`):

| | |
|---|---|
| integrated | **−18.6 LUFS** |
| true peak | **−6.1 dBFS** |

A game mix is usually mastered below −14 LUFS; **−18.6** is comfortably under with **6.1 dB** of true-peak headroom.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: does it hold, and can a developer understand it?

- **The two defects would both have passed a naive test.** Raw output had energy **0.494** at 1 kHz — a test asking
  "is there a 1 kHz tone" passes on a conversion displaced by most of a cycle. It took the **max error against a pure
  reference** (1.765444 → 0.071178) to see the displacement, and the **exact output length** (47600 → 48000) to see the
  short tail. Both numbers are in `research.md` and both are asserted.
- **The messages are readable.** A 44.1 kHz library now converts rather than refuses; the low-level
  `ResamplingRequired` still exists for a caller that skips the conversion, and says so.
- **The default build is untouched**: `cargo tree --edges normal` is **413 lines**, unchanged, and 174 tests pass with
  no features. Two ratchets fired on the way (`leaf` and `library`), which is them doing their job.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| issues | **#12 CLOSED** — the last one on the semantic critical path |
| new module | `src/semantic/resample.rs` |
| tests | **+11 resample, +1 decode rewritten**, from 271 to **282** all-features |
| loudness | **−18.6 LUFS**, true peak **−6.1 dBFS** |
| default graph | **413 lines**, unchanged |
