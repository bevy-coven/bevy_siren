# Iteration 78 — plan

## The item

[#6](https://github.com/bevy-coven/bevy_siren/issues/6): *"ADR + licence audit: model distribution and every model's
licence."* **p0**, **blocked by nothing**, and it blocks **#10, #39, #14**. It is a document, so the disk that has
blocked three consecutive iterations cannot stop it.

## Ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **#6 the model-licence ADR** | p0, unblocked, blocks three. It also **retires #39's premise**: #39 wants a music-specific encoder, and every candidate is non-commercial. Finding that out before #39 is built is worth more than building #39. |
| 2 | #16 incremental indexing by content hash | p0, blocked only by #11/#15 (both closed) — the next implementable p0. |
| 3 | #14 licence and source metadata | p0, but **blocked by #6** — this iteration unblocks it. |
| 4 | #5 the vector-store ADR | p0, unblocked, but it does not unblock a chain the way #6 does. |
| 5 | #21's index round trip | Waiting on `needs-decision/loudness-in-the-index.md`. |

**Why (1) beats (2) in one sentence:** #16 is buildable, but #6 decides whether a whole epic (#39, a music-specific
encoder) has a legal path at all, and it costs a document rather than a build — which is the only kind of work this
machine can currently finish.

## What the audit found, and why it changed the plan

Read from the Hugging Face API on 2026-11-03 (evidence in `evidence/model-licences.txt`):

| Model | Licence | Commercial |
|---|---|---|
| `laion/clap-htsat-fused`, `-unfused` | **Apache-2.0** | **yes** |
| `m-a-p/MERT-v1-95M`, `-330M` | **CC BY-NC 4.0** | **no** |
| `OpenMuQ/MuQ-large-msd-iter`, `MuQ-MuLan-large` | **CC BY-NC 4.0** | **no** |
| three other candidates | **no licence tag** | **no grant** |

**Every music-specific encoder on the shortlist is non-commercial.** That is a finding that changes a roadmap, not a
detail: #39 cannot ship as written.

And the corpus, which #6 insisted on: `laion/clap-htsat-fused` is Apache-2.0 **as weights**, but LAION-audio-630k has
**no licence file** (`404`, GitHub API `license: null`) and its README documents eight sources including purchased
commercial libraries. That is recorded as an open risk with a named escape hatch, not resolved by assertion.

## Decision

**CLAP only; no weights vendored; MERT and MuQ are blockers whose fallback is CLAP's text tower plus anchors.** Weights
arrive at **first use**, cached by content hash, pinned to a **commit sha**, verified against a **SHA-256 recorded in the
crate**, and every failure is a refusal naming the model. A project that forbids the download can consume an index as
data — `ModelStamp` and the hashed record format already make that possible.

## Modules touched

**None.** The deliverable is `docs/adr/0005-models-and-licences.md` plus its evidence. The ADR's "what this costs"
section records two *requirements* it creates — a pinned revision and a recorded hash — and they belong to whichever
issue builds the download path, not to this commit.

## Tests that prove it

An ADR is proved by the audit it records. Every licence line cites the API response that produced it and the date it was
read, and `evidence/model-licences.txt` is the raw output, so a later reader can re-run the query and check.

The one test-relevant finding is that **`ModelStamp` does not carry a pinned sha today** — its own doc says the version
*"names the measurement rather than a tag"*. The ADR states this as a gap rather than implying the stamp already does
what it does not.

## Carved off

- **Pinning the sha and recording the hash.** Real work, and it belongs with the fetch that uses it.
- **Any decision about which licences are acceptable for an *asset*.** That is `#14` and `S-05`, and this ADR is about
  *models*.
- **Downloading or shipping weights**, which #6 puts explicitly out of scope.
