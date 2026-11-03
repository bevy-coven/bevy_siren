# plan.md — iteration 74: CLAP's spectrogram, in Rust

## The pick, and why it beats the runner-up

**Pick:** the log-mel spectrogram — the mel path `indexer::build`'s seam has been waiting for, and iteration 73's first
item. It is **the largest blocker that is not a decision**: no model choice, no licence audit, no schema.

**Runner-up:** documentation. It is the cheapest item on the list and genuinely valuable, but a feature documented before
it works is a promise rather than a description.

**One sentence:** the indexer can now be given a real embedder, which turns the semantic layer from *every part present
and nothing assembled* into something a caller can actually run.

## The change

| Item | What it is |
|---|---|
| `mel::log_mel` | buffer → `[frames][64]` in dB, the extractor's pipeline |
| `mel::Filterbank` | the model's own 64 × 513 weights, loaded from data |
| `mel::fusion_channels` | `[4][1001][64]` — the four channels the graph takes |
| `mel::fft_in_place` | a 1024-point radix-2 FFT, thirty lines, no dependency |
| `Model::embed_audio` | runs the audio tower on those channels |

## Every number, and its source

**Every constant is read from the model**, not remembered: 48 000 Hz, 1024-sample window, 480 hop, 64 bands, 50–14 000 Hz,
513 bins, `1e-10` floor. `research.md` S2 has the table and S3 the pipeline read from `transformers`' own source.

## The tests that prove it

| Behaviour | Test |
|---|---|
| **the spectrogram equals the extractor's** | `the_spectrogram_matches_the_extractors_own_output` — within **7 × 10⁻⁶ dB** |
| the FFT is a DFT | `the_fft_matches_a_hand_computed_discrete_transform` — an impulse's spectrum is all ones |
| the window is periodic Hann | `the_window_is_periodic_hann` — sum is exactly n/2 |
| reflect padding excludes the edge | `reflect_padding_mirrors_without_repeating_the_edge` |
| 1 s gives 101 frames | `one_second_of_audio_gives_the_frame_count_the_pipeline_gives` |
| silence is −100 dB, not −∞ | `a_silent_buffer_sits_at_the_floor_not_at_negative_infinity` |
| a wrong-shaped filterbank is refused | `a_filterbank_of_the_wrong_shape_is_refused` |
| four channels of the model's size | `the_fusion_channels_are_four_windows_of_the_models_size` |
| determinism | `the_same_buffer_gives_the_same_spectrogram_twice` |

## What was carved off

**`Model::embed_audio` is written but not exercised against a real graph** — the 116 MB audio tower is on `bmb` and a
runtime check needs `ort` there. The spectrogram, which is the hard part and the part with numbers, is proved against
the extractor exactly.

**The three other items** — documentation, the weights schema, the tool host — are unchanged from iteration 73's list.
