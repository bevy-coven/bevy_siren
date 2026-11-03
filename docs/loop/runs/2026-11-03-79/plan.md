# Iteration 79 — plan

## The item

[#5](https://github.com/bevy-coven/bevy_siren/issues/5): *"ADR: vector index and metadata store."* **p0**, **blocked by
nothing**, blocks **#10** and **#15**. It is a document, which is the only kind of work this machine could finish: the
volume was at **148–449 MB free** and even a 7 KB `write` failed once with `ENOSPC`.

## Ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **#5 the vector-store ADR** | The last unblocked p0 that is a document. It decides where embeddings, features and metadata live — the store every other index issue writes into. |
| 2 | #16 incremental indexing | p0 and implementable, but needs a compiler and a baseline. |
| 3 | #14 licence and source metadata | p0, unblocked by iteration 78, also needs a build. |
| 4 | #21 index round trip | Waiting on `needs-decision/loudness-in-the-index.md` — and that decision is *about this store*. |
| 5 | #22 spectral features | Unblocked by ADR 0003, needs a build. |

**Why (1) beats (2) in one sentence:** #16 is the better build, and this machine cannot build — while #5 is the p0
that decides where #16's output goes, so finishing it means #16 starts on a settled format instead of discovering one.

## What the measurement corrected

The issue asks whether a game ships *"a 5 MB index as an asset or a 40 MB SQLite file"*. Computed from the crate's
**own** `to_text` output:

| Clips | Text (today) | Binary f32 | Binary f16 |
|---|---|---|---|
| 5 000 | **31.88 MB** | **10.30 MB** | 5.20 MB |

**The 40 MB figure is the format the crate already has**, and 5 MB needs `f16`. An ADR written from the issue's
numbers would have chosen a store to avoid a size it is already at.

Query cost, measured: **0.05–1.87 ms** for a BLAS `f32` dot over 5 000 × 512. Brute force is not the slow option here.

## Decision

**One file, read whole, brute force.** No SQLite (a C dependency to solve 10 MB of write amplification), no shard
directory (thousands of `open`s and no single checksum), no ANN (unnecessary below 50 000 by the measurement). Binary
`f32` is what ships; the text format stays as the debuggable form.

**Identity is the content hash**, with four named rules: rename keeps the vector, re-encode is a new asset, duplicate
bytes are one vector with two paths, and a hash collision is **detected** by the stored size rather than assumed away.

**A query needs no writable filesystem**, stated as the issue requires.

## Modules touched

**None.** The deliverable is `docs/adr/0004-vector-store.md` and its evidence. The ADR's "what this costs" section
records what implementing it will change in `semantic::index` — a binary sibling to `to_text`/`load`, and the same
`MAGIC` discipline so a file is identifiable either way.

## Tests that prove it

An ADR is proved by the numbers it records. Every size in it is arithmetic on a **measured** per-record cost
(6 376.8 bytes over 2 000 records) and a measured dimension (512), and every latency is a measurement on this machine
with the extrapolation to 50 000 labelled as extrapolation.

The one test-relevant finding: the existing text format is **3× larger** than the binary form the crate should ship,
which is a fact about `to_text` that no test asserts today.

## Carved off

- **Implementing the binary form.** It needs a build and a baseline; this ADR fixes its shape.
- **`f16`.** Measured, preserved one query's top 10, and deferred to `V-01` because it changes the precision at which a
  score meets `rank::top_k`'s floor.
- **The loudness section question** (`needs-decision/loudness-in-the-index.md`) — that is a schema decision *inside*
  this store, and it stays a decision file rather than being folded in here.
