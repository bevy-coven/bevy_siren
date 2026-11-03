# research.md — iteration 65: the resampler, and two things the probe missed

## S1 — `rubato 0.16.2`, measured again with the flush and the latency

Iteration 63 measured that `FftFixedInOut` keeps a tone intact but reported a **480-sample** shortfall, which it
attributed to an unflushed final chunk. Both halves of that were incomplete. This iteration measured the whole path.

| Quantity | Measured |
|---|---|
| 1.000 s at 44.1 kHz | **44100** samples in |
| ratio to 48 kHz | 48000 / 44100 = 1.08844 |
| output without a fix | **47600** samples of real audio, 400 of padding |
| `Resampler::output_delay()` | **560 samples** at 44.1 → 48 kHz, chunk 1024 |
| max error vs a pure 1 kHz reference, **raw** | **1.765444** — off by a whole cycle, i.e. meaningless |
| max error after skipping the delay | **0.071178** — a real match |
| energy at 1 kHz after the fix | **0.5000** in, **0.5000** out |

### The technique: skip the delay, then hold the length

An FFT resampler cannot emit its first output until its window is full. `rubato` reports that cost as
[`Resampler::output_delay`], and **skipping it is what makes the output line up with the input**. Without the skip the
signal is displaced by most of a cycle, which still *looks* like a tone — the energy at 1 kHz is 0.494 — so a test that
only checked "is there a 1 kHz tone" would pass on a broken conversion.

### The second defect: stopping when the input is spent

The first implementation looped `while pos < mono.len()`. With a 560-sample delay to pay and a padded final chunk, that
leaves the tail short: **47600** samples instead of 48000. The symptom was subtle — the tone came out at **991 Hz**
instead of 1000, because 400 samples of the last cycles were zeros.

**The corrected loop runs until `expected + delay` samples exist**, regardless of whether the input is spent. Both
numbers are checkable by hand: a 1.000 s input is **exactly 48000** samples out, and a 1 kHz tone is still 1 kHz.

## S2 — what the literature says, and why the measurement mattered more

`hs distill search` on resampling returned nothing directly about audio sample-rate conversion: the nearest hits were
*Physically Based Rendering* on sampling and reconstruction, and *Linear Interpolation Revitalized* (DOI
10.1109/TIP.2004.826093), which is about image warping and speech, not audio rate conversion.

**That is a real gap, and it is recorded rather than papered over.** The empirical basis here is the crate's own
measurement: three converters, one tone, one number each.

| Converter | 1 kHz through 44.1 → 48 kHz | Source |
|---|---|---|
| `rodio`'s `SampleRateConverter` | **not measured — ruled out by its own doc comment**: linear interpolation up, discarded samples down | `rodio-0.22.2/src/conversions/sample_rate.rs` |
| hand-rolled linear interpolation | kept the tone, band-limited badly; **removed** in iteration 62 | this crate's own history |
| **`rubato` `FftFixedInOut`** | **0.5000 of 0.5000** energy at 1 kHz, 15 kHz intact, delay 560 skipped | measured, S1 |

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| chunk size | **1024** frames, a power of two for the FFT | `rubato`'s own example |
| output delay, 44.1 → 48 kHz | **560** samples | `Resampler::output_delay()`, measured |
| output delay, 48 → 44.1 kHz | measured per call, not assumed | same API |
| 1 kHz energy through the conversion | **0.5000 of 0.5000** | measured, S1 |
| output length for 1.000 s | **48000** exactly | measured, S1 |
| max error vs a pure reference | **0.071178** | measured, S1 |

## What good sounds like

For a conversion, "good" is three things and all three are numbers: **the duration is the duration** (1.000 s stays
1.000 s), **the frequency is the frequency** (1 kHz stays 1 kHz, ±2 cycles counting crossings), and **the energy is the
energy** (0.5 stays 0.5 within 5 %). The fourth — no image energy where the fold would put it — is checked by asserting
15 kHz survives and 9.1 kHz does not appear.

The failure this guards against is specific: an aliased or displaced conversion still produces a mel spectrogram, still
produces a plausible embedding, and **fails nothing downstream**. That is why the numbers are asserted rather than
inspected.
