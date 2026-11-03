# research.md — iteration 75: the last closure, and the documentation that was missing

## S1 — the two items, and why both

Iteration 74's list had two items at the top, and they are different in kind:

| Item | Why it mattered |
|---|---|
| **wire the mel path into `build`** | the chain was one closure short of runnable: `mel::input_features` → `Model::embed_audio` |
| **documentation** | the semantic layer had **zero** README mentions and no guide page — measured, `grep -ci semantic README.md` was **0** |

A loop that ships a feature nobody can find has shipped half of it. Both landed.

## S2 — the closure, and the one thing it had to decide

`build_from_model(root, model, filterbank, stamp)` fills [`indexer::build`]'s seam:

```text
decode → resample → mel::input_features → Model::embed_audio → Index::insert
```

The one decision is **`is_longer`**, the audio tower's second input. The extractor sets it per clip from whether the
audio exceeded the window. Here it is read from the **buffer's length** — `buffer.len() > MODEL_WINDOW_SAMPLES` — and
**reported** on `Built::longer` rather than assumed, because it selects a different branch of the graph and a caller
comparing two indexes needs to know whether one took it.

## S3 — what the end-to-end test proves, and what it does not

`a_folder_builds_an_index_with_the_models_own_spectrogram` runs the whole chain with the **real mel path** and a
**stand-in embedder** that summarises the four channels into a 512-d vector. It proves:

- decode and resample feed the mel path;
- the mel path produces `[4][1001][64]`;
- a vector from that enters the index under the content hash;
- two files of different rate and length both survive the chain.

It does **not** prove the 116 MB audio tower runs, because that graph lives on `bmb` and needs `ort` there. That
distinction is in the test's own comment and in the guide page's status line, rather than left for a reader to discover.

## S4 — the documentation, and it is honest about the gap

`docs/guide/semantic.md` is a page a developer can act on:

| Section | What it says |
|---|---|
| status | **"not yet end-to-end"**, and which link is unverified |
| the four steps | a diagram of what each stage is, with the *measured* numbers |
| the floor | the recall table — 100 % at 0.0, **0 %** at 0.5 — because that is why the default is what it is |
| the ensemble | the four-facts-per-signal rule and the sentence it produces |
| tuning | `Weights`, and the line between *impossible* and *unusual* |
| what it does not do | no device, no weight download, no licence claims |

The README gains one bullet, placed with the other "what you can do" entries and linking the page.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| semantic mentions in the README, before | **0** | measured |
| semantic mentions in the README, after | **2** | measured |
| guide pages, before | 5 | `docs/guide/` |
| guide pages, after | **6** | measured |
| the chain's steps | **5** | decode, resample, mel, embed, insert |
| agreement of the mel path with the model | **≤ 7 × 10⁻⁶ dB** | measured, iteration 74 |

No new constants. The closure adds one `bool` and one function; the guide adds numbers already measured and cited.

## What good sounds like

For documentation, "good" is that a developer learns **what works, what does not, and how they would know** — in that
order, before the API. The status line says *"the audio tower has never been run against the real model in this
repository"* and then says exactly which numbers make the rest trustworthy: the spectrogram is exact against CLAP's
extractor, the tokeniser is exact against the model's, and the floor is measured. A page that led with the API and
buried that would be worse than no page.
