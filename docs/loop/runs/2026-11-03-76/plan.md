# Iteration 76 — plan

## The item

Iteration 75 left **one** thing at the top of its list: *"Run the audio tower against the real graph on `bmb`… This is a
verification pass, not a build."* Its own log called it *"the only unverified link in the chain, and the guide page says
so in its first paragraph."*

The pass was left half-done in the working tree: code edits in `src/semantic/mel.rs` and `model.rs` and a single
`evidence/audio-tower.txt` whose own conclusion is **`cosine 0.850, not a working feature`**. Under the loop's first
rule — *finish or revert it first; a new item on top of a broken one measures nothing* — the half-done pass **is** this
iteration.

## Ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **Finish the audio-tower verification: make the embedding match the reference** | Category 1. The semantic layer's whole audio path is *wrong output* right now, and iteration 75 shipped a guide page telling readers it works. A green test suite over a wrong embedding is the exact failure mode this loop exists to catch. |
| 2 | `#28` the MCP tool host | Category 4, and blocked on nothing — but it consumes an index that is not yet correct. |
| 3 | `#29` `search_music`, `#44` `explain_match` | Built on the same embedding. |
| 4 | `V-01` the labelled set | Needs a person. |
| 5 | `cargo run --example semantic` | The artifact that proves the path; it should show the *right* numbers, so it follows item 1. |

**Why (1) beats (2) in one sentence:** every remaining semantic item consumes the audio embedding, so shipping the MCP
host over an embedding at cosine 0.85 would put a wrong answer behind a polished tool — the "ships nothing is a failed
pass" rule inverted.

## Design

No new public API, no new field, no new refusal. Three corrections, each with the reference line it comes from:

| Change | Module | Before | After | Source |
|---|---|---|---|---|
| the right-hand reflect pad is `WINDOW/2`, not `needed − len` | `mel::log_mel` | `right = needed_right − buffer.len()`, i.e. **1024** | `right = needed − (len + half)`, i.e. **512** | `np.pad(waveform, (512, 512), "reflect")` |
| the FFT runs in `f64` | `mel::log_mel`, `mel::fft_in_place` | `f32` throughout | `f64` accumulate, `f32` output | `spectrogram` does `waveform.astype(np.float64)` |
| `is_longer` is a **bool** tensor | `model::embed_audio` | `Vec<i64>` | `Vec<bool>` | the graph declares `tensor(bool)` |
| every semantic error implements `std::error::Error` | all 12 `src/semantic/*.rs` | `Display` only | `Display` + `Error` | `?` in a `Box<dyn Error>` caller |

The `fusion_channels` rewrite (tile the waveform, stack one mel four times) was already in the tree from the half-done
pass and is kept: it is the extractor's behaviour, and iteration 76's evidence proves the tiling by measurement.

## Tests that prove it

1. **`the_tiled_model_window_matches_the_extractors_own_output`** — frames **0, 500 and 1000** of the tiled 480 000-sample
   window against three rows measured from the pretrained extractor, tolerance **0.06 dB** (measured max 0.0545). Frame
   1000 is the one the old code could not reach: it is the last frame, and a short pad puts it at −100 dB instead of
   ~16 dB, a 116 dB error at one index.
2. **`no_frame_of_the_tiled_window_falls_to_the_floor`** — the count of all-floor frames is **0**, the number the
   reference reports for a tone that tiles seamlessly. This is the shape of the original bug: a hole, not a drift.
3. `the_spectrogram_matches_the_extractors_own_output` (iteration 74's, unchanged) stays as the single-window guard.
4. The twelve `std::error::Error` impls are compile-level: a `Box<dyn Error>` caller with `?` is the test, and it is
   what failed to compile on `bmb`.

## Carved off

- **Chunking a track longer than 480 000 samples.** `input_features` refuses to guess; how a 3-minute file becomes
  10-second windows is `I-03`'s decision with its own numbers. The function's doc says so.
- **A `cargo run --example semantic`.** It would be the right artifact, and it is item 5 on the ranked list; this
  iteration measures the same path from a probe and leaves the example to a pass that is not repairing a defect.
