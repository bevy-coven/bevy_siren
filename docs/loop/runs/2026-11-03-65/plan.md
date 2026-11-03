# plan.md — iteration 65: #12 closes, and the resampler's two defects

## The pick, and why it beats the runner-up

**Pick:** `#12`'s remaining half — the conversion. It is the **last open issue on the semantic critical path**, and it
is the one whose absence made the feature look broken for the commonest asset rate.

**Runner-up:** `#41` (tunable weights). It loses on dependency order: `#41` refines a ranking, and a ranking is only
useful once a library can be indexed at all. It also loses on the loop's ranking — a **core behaviour** gap outranks
authoring ergonomics.

## What changed, and the two defects it took to get there

`rubato 0.16.2` is in the crate behind `semantic`. The first implementation was wrong in two ways, and both are worth
recording because both produce a *plausible* result:

| Defect | Symptom | Fix |
|---|---|---|
| the resampler's **latency** was not skipped | the tone was displaced by most of a cycle; energy at 1 kHz was still **0.494**, so a naive test would pass | skip `Resampler::output_delay()` — **560** samples at 44.1 → 48 kHz |
| the loop stopped when the **input** was spent | **47600** samples instead of 48000, so the tone read as **991 Hz** | run until `expected + delay` samples exist |

Max error against a pure reference went from **1.765444** (raw) to **0.071178** (after the skip) — that is the number
that shows the latency fix was the real one.

## Every number, and its source

| Number | Value | Source |
|---|---|---|
| output delay, 44.1 → 48 kHz | **560** samples | `Resampler::output_delay()`, measured |
| chunk size | **1024** frames | `rubato`'s own example, a power of two for the FFT |
| 1 kHz energy, in and out | **0.5000 / 0.5000** | measured, `research.md` S1 |
| 1.000 s at 44.1 kHz | **exactly 48000** at 48 kHz | measured |
| max error vs a pure reference | **0.071178** | measured |
| default dependency graph | **413 lines**, unchanged | measured |

No magic constant: `CHUNK` cites `rubato`'s example size, and the delay is read from the library rather than assumed.

## The tests that prove it

| Behaviour | Test |
|---|---|
| 1.000 s becomes exactly 48000 samples | `semantic_resample::one_second_at_44_1k_becomes_exactly_48000_samples` |
| the tone keeps its frequency | `semantic_resample::the_tone_keeps_its_frequency_through_the_conversion` |
| the tone keeps its energy | `semantic_resample::the_tone_keeps_its_energy_within_the_measured_band` |
| nothing folds to an image | `semantic_resample::a_high_tone_survives_rather_than_aliasing` |
| awkward lengths keep their duration | `semantic_resample::a_length_that_is_not_a_multiple_of_the_chunk_keeps_its_duration` |
| down-sampling too | `semantic_resample::down_sampling_also_keeps_its_duration` |
| a 44.1 kHz file reaches the model | `semantic_decode::a_file_at_another_rate_is_converted_not_refused` |
| conversion is deterministic | `semantic_resample::the_conversion_is_deterministic` |

## What was carved off

Nothing from `#12` — it closes. What remains on the crate's semantic path is unrelated: a **tokeniser** (#41's
neighbour), **CUDA** for throughput, and the **labelled set** that would turn the rank floor from a design default into
a measurement.
