# 0005 — Models and licences: what may ship in a game toolchain

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#6](https://github.com/bevy-coven/bevy_siren/issues/6)
- **Blocks:** [#10](https://github.com/bevy-coven/bevy_siren/issues/10),
  [#39](https://github.com/bevy-coven/bevy_siren/issues/39),
  [#14](https://github.com/bevy-coven/bevy_siren/issues/14)
- **Evidence:** [`loop/runs/2026-11-03-78/research.md`](../../loop/runs/2026-11-03-78/research.md)

## The question

A game toolchain cannot ship a non-commercial model. Which models may this crate depend on, and how do their weights
reach a user who builds a game with it?

Every licence below was read from the model's own card **on 2026-11-03**, via the Hugging Face API's `license:` tag or
the card text. None is carried from recollection; #6 says so explicitly, and the audit below is only worth its date.

## The audit

| Model | Weights licence | Commercial use | Read from |
|---|---|---|---|
| `laion/clap-htsat-fused` | **Apache-2.0** | **yes** | HF API tag `license:apache-2.0`; card frontmatter |
| `laion/clap-htsat-unfused` | **Apache-2.0** | **yes** | HF API tag |
| `m-a-p/MERT-v1-95M` | **CC BY-NC 4.0** | **no** | HF API tag `license:cc-by-nc-4.0` |
| `m-a-p/MERT-v1-330M` | **CC BY-NC 4.0** | **no** | HF API tag |
| `OpenMuQ/MuQ-large-msd-iter` | **CC BY-NC 4.0** | **no** | HF API tag |
| `OpenMuQ/MuQ-MuLan-large` | **CC BY-NC 4.0** | **no** | HF API tag |
| `minzwon/musicfm-v-demo`, `nvidia/audio-flamingo-hf`, `EffOrg/EF-CLAP` | **no licence declared** | **unknown** | HF API returns no `license:` tag |

**Three of the four candidate families are non-commercial.** MERT and both MuQ variants — the entire music-style
encoder shortlist — are CC BY-NC 4.0. "No licence declared" is not permissive either: an unlicensed weight grants no
rights.

### The corpus, which the issue was right to insist on

`laion/clap-htsat-fused` is Apache-2.0 **as weights**, and trained on **LAION-audio-630k**. That corpus's own README
states it was assembled from eight sources including **Epidemic Sound**, **Audiostock**, **Sonniss Game Effects**,
**Free To Use Sounds**, **We Sound Effects** and **Paramount Motion Sound Effects**, several of which LAION
**purchased**, and that only four of the eight are released.

The `LAION-AI/audio-dataset` repository returns **HTTP 404 for its `LICENSE` path** and the GitHub API reports
**`license: None`** — there is no licence file on the corpus repository at all.

This is a **material risk, recorded and not resolved**: a permissively-licensed checkpoint trained on a corpus with no
stated licence, containing purchased commercial sound libraries. It is not a reason to stop using CLAP — LAION released
the weights as Apache-2.0, which is the grant a downstream consumer relies on — but it is the reason the fallback below
exists and the reason a project with a strict legal review must be able to replace the model.

## Decision

**CLAP only. The music-specific encoder is dropped.** No weights are vendored.

1. **`laion/clap-htsat-fused` is the one model**, under `docs/adr/0002-inference-runtime.md`'s ONNX export. It is
   Apache-2.0 and the only candidate family whose licence permits commercial use.
2. **MERT and MuQ are blockers, not options.** Both are CC BY-NC 4.0. The fallback when a music-style layer is needed
   is **CLAP's own text tower plus the anchor mechanism** — describe or point at a track and let the ensemble rank from
   acoustic features — which is strictly weaker for music and is the honest cost of the licence. `#39`'s music-specific
   encoder does not ship until a commercially-licensed one exists and is audited.
3. **No weights in the crate.** The crate downloads to a cache directory; the repository never contains a checkpoint.

## How weights reach a user

| | |
|---|---|
| **origin** | `https://huggingface.co/laion/clap-htsat-fused`, or the ONNX export described in ADR 0002 |
| **when** | **at first use**, not at build time — a project that never queries never downloads |
| **cache** | the platform's cache directory, keyed by **model name and a content hash**, never by a version string alone |
| **pinning** | a **commit sha in the Hugging Face repo**, recorded in the index's `ModelStamp` |
| **integrity** | the downloaded file's **SHA-256 compared against a value recorded in the crate**, not against one served beside it — a hash delivered with the payload verifies nothing |
| **failure** | a download that fails, or a hash that mismatches, is a **refused** operation naming the model, the expected hash and the URL. There is no silent fallback to a different model: comparing vectors across models is the failure `ModelStamp` exists to prevent |

**A gap this ADR exposes in existing code.** `semantic::index::ModelStamp` today has `model`, `version`, `sample_rate`
and `dimension`, and `ModelStamp::measured_clap()`'s own doc admits the gap: *"The version names the measurement rather
than a tag, because the spike did not pin a revision"* — it stores `MEASURED_CLAP_VERSION`, a crate constant, not a
sha. So the pinning story above is a **requirement on the download path**, not a description of what exists. Until a
sha is pinned, two builds of this crate could load different weights under the same stamp, which is precisely the
comparison `ModelStamp` was built to refuse.

## What happens when a user cannot or must not download

Three states, each named rather than degraded into something that looks fine:

| Situation | Behaviour |
|---|---|
| offline, nothing cached | the semantic query is **refused** with the model named; playback, mixing and every non-semantic feature are unaffected, because `semantic` is off by default |
| hash mismatch | **refused**, with both hashes in the message |
| a project forbids the download | the crate reads an existing index as **data** — option (d) below — and ranking works against vectors someone else built |

**Option (d), consumed as data, is real and supported.** The index already stores vectors keyed by content hash with a
`ModelStamp`; a project can ship that file and never load a model at all. That is the path for a studio whose legal
review rejects even the Apache-2.0 weights, and it is why the licence risk above does not make the feature unusable.

## Rejected options

| Option | Why not |
|---|---|
| **(b) CLAP plus MuQ** | MuQ is **CC BY-NC 4.0**. It cannot ship in a game toolchain. |
| **(c) MERT if a commercial licence exists** | It does not: `m-a-p/MERT-v1-95M` and `-330M` are both CC BY-NC 4.0. |
| **Vendoring the weights** | 116.5 MB in the crate's own package, for a feature that is off by default. Cargo packages are cached and mirrored; a 116 MB payload in one is a cost every consumer pays and no consumer can opt out of. |
| **Downloading at build time** | `ort` already downloads a runtime at build time behind `semantic`, and that is the one build-time download this crate tolerates. A second turns `cargo build` into a network operation with two failure modes. |

## What this costs

- **`#39`, a p1 music-specific encoder, does not ship.** That is the largest feature cost in this ADR and the reason the
  issue is p0: it had to be decided before #39 was built, not after.
- **`#14` can proceed**, because it only needed to know what may be recorded — provenance per asset, with an explicit
  unknown, is unaffected by which models are legal.
- **`#10` can proceed**: extending `tests/leaf.rs`'s closed dependency list is unaffected by which model family is
  dropped, since the crate declares no new dependency either way.
- **A pinned revision and a recorded hash are new requirements** on the download path, and `ModelStamp` does not carry
  a sha today. That work belongs with whichever issue builds the fetch, and it must land before an index is trusted
  across two machines.

## The one open risk, stated so it is not buried

The corpus has **no licence file**, and includes purchased commercial libraries. The weights are Apache-2.0, which is
the grant a downstream user relies on, and this crate does not redistribute them. A studio with a strict review is
directed to **option (d)** rather than told the question is settled.
