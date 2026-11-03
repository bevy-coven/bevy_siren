# plan.md — iteration 64: the decode half of #12

## The pick, and why it beats the runner-up

**Pick:** `#12`'s decode half — a file, decoded to interleaved samples, plus the rate and channel count the stream
reports, and one function from a path to a model buffer.

**Runner-up:** `#41` (tunable weights as a serde type). It is a smaller, safer item, and it loses on the loop's own
ranking: a feature that cannot read the commonest asset rate is a **core behaviour** gap, while tunable weights are
authoring ergonomics. It also loses on dependency order — `#41` refines a ranking that `#12` decides whether can run
at all.

## What was carved off, and it is stated rather than hidden

**Carved off: the conversion.** `#12` says "decode **and resample**". Decoding is built; resampling is not, because it
needs a fifth dependency and the crate's rule sends that to `needs-decision/`. `resampler-dependency.md` records four
options, three measurements, and a recommendation — `rubato` — with the caveat that would bite (the final partial
chunk must be flushed; the probe shows a 480-sample gap).

**The refusal is the deliverable, not a placeholder.** A 44.1 kHz file now decodes successfully and is then refused
with both rates named. That is a different message from "your file is broken", and it is the truthful one.

## Every number, and its source

| Number | Value | Source |
|---|---|---|
| `rodio` already in the default graph | **yes**, via `bevy_audio` | measured, `research.md` S1 |
| default graph size, unchanged | **413 lines** | measured, S1 |
| a 440 Hz tone for 0.5 s | exactly **220** rising zero crossings | measured, S2 |
| a 48 kHz 0.1 s file | 4800 frames | measured, S2 |
| per-frame mix, 8 / 64 / 256 voices | **0.024 / 0.284 / 0.667 µs** | measured, `perf.md` |

No magic constant. The one number added to `src/` is a doc comment quoting the measurement, and the refusal carries
the file's own rate rather than a constant.

## The tests that prove it

| Behaviour | Test |
|---|---|
| a WAV decodes to its own rate and channels | `semantic_decode::a_wav_decodes_to_its_own_rate_and_channels` |
| the decode reproduces the **signal** | `semantic_decode::the_decoded_audio_is_the_tone_that_was_written` |
| 48 kHz reaches a model buffer | `semantic_decode::a_file_at_the_models_rate_goes_all_the_way_to_a_model_buffer` |
| 44.1 kHz decodes **and then** is refused | `semantic_decode::a_file_at_another_rate_is_decoded_and_then_refused_naming_both_rates` |
| a missing file names the path | `semantic_decode::a_missing_file_says_which_file` |
| a non-audio file is refused, not a panic | `semantic_decode::a_file_that_is_not_audio_is_refused_rather_than_panicking` |
| the feature names exactly two dependencies | `library::the_semantic_feature_declares_no_dependency_of_its_own` |
| the dependency list is five and argued | `leaf::the_dependency_list_stays_closed` |

## The pin that caught me

`the_semantic_feature_declares_no_dependency_of_its_own` failed the moment `semantic` grew `dep:rodio` — which is
exactly what it exists for. It was updated with the argument in the same commit: `rodio` costs **no new crate**,
because `bevy_audio` resolves it already.
