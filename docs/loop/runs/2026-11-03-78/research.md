# Iteration 78 — research: model licences, and what a game toolchain may ship

The item is [#6](https://github.com/bevy-coven/bevy_siren/issues/6): *"ADR + licence audit: model distribution and
every model's licence."* p0, nothing blocks it, blocks #10, #39 and #14. Deliverable:
`docs/adr/0005-models-and-licences.md`.

## Sources — each read on 2026-11-03, none from recollection

#6 says *"Verify each licence from the model card at audit time. Do not carry a recollection into the file."* Every line
below is the Hugging Face API's own `license:` tag or the card's frontmatter, recorded in
`evidence/model-licences.txt`.

| Source | The number or fact it reports |
|---|---|
| `huggingface.co/api/models/laion/clap-htsat-fused` | **`license:apache-2.0`**, not gated |
| `.../laion/clap-htsat-unfused` | **`license:apache-2.0`** |
| `.../m-a-p/MERT-v1-95M` | **`license:cc-by-nc-4.0`** |
| `.../m-a-p/MERT-v1-330M` | **`license:cc-by-nc-4.0`** |
| `.../OpenMuQ/MuQ-large-msd-iter` | **`license:cc-by-nc-4.0`** |
| `.../OpenMuQ/MuQ-MuLan-large` | **`license:cc-by-nc-4.0`** |
| `.../minzwon/musicfm-v-demo`, `nvidia/audio-flamingo-hf`, `EffOrg/EF-CLAP` | **no `license:` tag at all** |
| `raw.githubusercontent.com/LAION-AI/audio-dataset/main/LICENSE` | **HTTP 404** — no licence file |
| `api.github.com/repos/LAION-AI/audio-dataset` → `.license` | **`null`** |
| LAION-audio-630k README | assembled from **8 sources**; **only 4 released**; includes **Epidemic Sound, Audiostock, Sonniss Game Effects, Paramount Motion Sound Effects**, several **purchased by LAION** |
| `laion/clap-htsat-fused` card | `license: apache-2.0`; trained on `LAION-audio-630k` |
| this crate, `src/semantic/index.rs::ModelStamp` | `model`, `version`, `sample_rate`, `dimension`; `measured_clap()`'s doc: *"the version names the measurement rather than a tag, because the spike did not pin a revision"* |

## The technique, in the sources' terms

Three separate grants have to line up for a game to ship a model, and the issue is right that they are usually
conflated:

1. **Weights licence** — the grant on the checkpoint file itself. This is what a downstream consumer legally relies on.
2. **Training-corpus licence** — a permissive checkpoint trained on a restricted corpus is the classic trap #6 names.
   The corpus here has **no licence file at all**, and its README documents purchased commercial libraries inside it.
3. **Distribution method** — vendoring, build-time download, or first-use download. Each turns a different failure
   into a different user experience, and only one of them keeps `cargo build` a local operation.

## Parameters the sources report

| Parameter | Value | Where |
|---|---|---|
| CLAP weights | **Apache-2.0** | HF API tag |
| MERT weights (both sizes) | **CC BY-NC 4.0** | HF API tag |
| MuQ weights (both variants) | **CC BY-NC 4.0** | HF API tag |
| unlicensed candidates | **no grant** | HF API: no tag |
| corpus licence | **none stated** | 404, `null` |
| corpus sources | 8, of which **4 released** | corpus README |
| CLAP ONNX audio tower | 116.5 MB | ADR 0002 |

## What good sounds like

A decision that can be audited: every model named with a licence, a URL and the **date read**; any non-commercial
model marked a **blocker with a named fallback**, not quietly omitted; and a distribution story where the *failure*
paths are specified too — offline, hash mismatch, and a project that forbids the download. The last of those is what
makes an Apache-2.0-but-unclear-corpus model acceptable at all: a studio must be able to consume an index built
elsewhere and never load weights, and the crate already has the `ModelStamp` and hashed record format that makes that
possible.
