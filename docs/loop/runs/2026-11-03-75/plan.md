# plan.md — iteration 75: the closure, and the documentation

## The pick, and why both

**Pick (code):** `build_from_model` — the closure that joins `mel::input_features` to `Model::embed_audio`, so a folder
becomes a stamped index with no caller-supplied embedder.

**Pick (docs):** `docs/guide/semantic.md` and a README bullet.

**Why both, in one iteration:** they are the same item seen twice. The code makes the feature runnable; the page makes
it *findable*. Iteration 74 measured the semantic layer at **zero** README mentions — a feature nobody can find is not
shipped, and this repository's own rule says a feature is not done until its README line exists.

**Runner-up:** exercising the audio tower against the real 116 MB graph. That needs a GPU host and `ort` there, so it is
a verification pass rather than a build.

## The change

| Item | What it is |
|---|---|
| `indexer::build_from_model` | decode → resample → mel → embed → insert, with the model doing the embedding |
| `Built::longer` | whether any file exceeded the model's window, **reported** rather than assumed |
| `docs/guide/semantic.md` | the six sections a developer needs, status first |
| `README.md` | one bullet, beside the other capabilities |
| `docs/guide/README.md` | the index entry |

## Every number, and its source

No new constants. `is_longer` is derived from `buffer.len() > MODEL_WINDOW_SAMPLES`, which is a measured constant
(`nb_max_frames × hop_length = 1000 × 480`). Every number in the guide is one this crate measured and cited in an
earlier iteration.

## The tests that prove it

| Behaviour | Test |
|---|---|
| **the whole chain, with the real mel path** | `a_folder_builds_an_index_with_the_models_own_spectrogram` |
| a 1 s and a 0.5 s file both survive it | same test — 2 indexed, 0 failed |
| `longer` is false for files inside the window | same test |

## What was carved off, and it is the one gap left in the chain

**No run of the audio tower against the real graph.** `Model::embed_audio` is written and type-checked; the 116 MB
`clap-audio.onnx` lives on `bmb` and a runtime check needs `ort` there. The guide's status line says so in its first
paragraph rather than implying otherwise, which is the honest form of the same admission.
