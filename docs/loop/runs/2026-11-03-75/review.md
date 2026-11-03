# review.md — iteration 75, three passes

## Pass 1 — Function: does the chain run, and can it be found?

| Claim | Test | Result |
|---|---|---|
| the whole chain, real mel path | `a_folder_builds_an_index_with_the_models_own_spectrogram` | **2 indexed, 0 failed** |
| `longer` false for files in the window | same test | asserted false |
| the semantic layer is documented | `grep -ci semantic README.md` | **0 → 2** |
| the guide index links it | `grep "Semantic selection" docs/guide/README.md` | 1 |
| the audio path is undisturbed | the visual gate | **−21.5 LUFS, −7.5 dBFS**, 14 distinct cues — identical to iterations 67/71/74 |

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Three defects, all found by the compiler or clippy rather than by me:

| Defect | Found by | Fix |
|---|---|---|
| `Built` gained a field and **one initialiser was missed** | the compiler | `longer: false` added to `build`'s return |
| a `let mut run` that needed no `mut` | the build warning | removed — **but my first fix removed it from `embed_text` too**, where it is required |
| `let mut built` in the new test | clippy | removed |

The second is worth naming: a blanket text replace made `embed_text`'s closure non-`mut`, the compiler refused
(`cannot borrow run as mutable`), and the fix was to restore `mut` there and remove it only from `embed_audio`, whose
closure clones rather than moves. **A global edit that happens to compile in one place can be wrong in another** —
and the compiler caught it.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use this without me?

Open the guide page cold. The first paragraph says:

> *"Status: behind the `semantic` feature, and not yet end-to-end … the audio tower has never been run against the real
> model in this repository."*

Then it says which numbers make the rest trustworthy: the spectrogram is exact against CLAP's extractor (0 and 4 × 10⁻⁶
on two bands), the tokeniser is exact against the model's, and the floor's table shows why 0.0 keeps everything while
0.5 keeps nothing.

- **A developer learns what works, what does not, and how they would know** — in that order, before the API.
- **Every number in the page was measured by this loop** and cites the iteration it came from.
- **The one unverified link is named twice**: in the status line and in the page's closing section.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| items | `build_from_model`, and the documentation the layer never had |
| tests | **+1**, from 367 to **368** all-features |
| docs | 1 new guide page, 2 README/guide-index edits, **0 → 2** README mentions |
| visual gate | unchanged: −21.5 LUFS, −7.5 dBFS, 14 distinct cues |
| carved off | running the audio tower against the real graph (needs `ort` on `bmb`) |
