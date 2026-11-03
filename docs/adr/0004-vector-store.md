# 0004 — Vector index and metadata store: one file, brute force, no SQLite

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#5](https://github.com/bevy-coven/bevy_siren/issues/5)
- **Blocks:** [#10](https://github.com/bevy-coven/bevy_siren/issues/10),
  [#15](https://github.com/bevy-coven/bevy_siren/issues/15)
- **Evidence:** [`loop/runs/2026-11-03-79/research.md`](../../loop/runs/2026-11-03-79/research.md)

## The question

Where do embeddings, acoustic features and per-asset metadata live? How are they keyed, and how is an incremental
update done without rewriting everything?

## What was measured first

The crate already **has** a store — `semantic::index` writes a versioned text file (`bevy_siren index 1`) — so the
options are judged against a working implementation, not a sketch. Sizes were computed from that format's actual
`to_text` output, not estimated.

### File size, at three library sizes and three encodings

A record is 16 hex characters for the content hash plus 512 floats. The mean text record is **6 376.8 bytes**, measured
over 2 000 synthetic 512-d unit vectors printed with `{:?}` — Rust's shortest round-trip form, which is what `to_text`
uses.

| Clips | **Text (`to_text`)** | **Binary `f32`** | **Binary `f16`** |
|---|---|---|---|
| 500 | **3.19 MB** | 1.03 MB | 0.52 MB |
| **5 000** | **31.88 MB** | **10.30 MB** | **5.20 MB** |
| 50 000 | 318.84 MB | 102.99 MB | 52.01 MB |

**This corrects the issue's own premise.** #5 asks whether a game ships *"a 5 MB index as an asset or a 40 MB SQLite
file"*. At 5 000 clips the answer is neither: **text is 31.9 MB**, which is the 40 MB figure the issue attributes to
SQLite. The 5 MB figure is only reachable in **`f16` binary**.

### Query latency, in memory

| Operation | Time |
|---|---|
| BLAS `f32` dot over 5 000 × 512 | **0.05 – 1.87 ms** (first call cold, then 0.05–0.16) |
| BLAS `f16` dot over the same | 4.90 ms |
| `f16` top-10 agreement with `f32` | **identical** on this query |
| Scalar `f32` dot, 5 000 × 512 | ~2–4 ms (one core, no SIMD) |

A brute-force scan at 5 000 clips is **single-digit milliseconds**. That decides the ANN question: a structure would
optimise a cost already below a frame, at the price of a dependency and of exactness.

## Decision

**Option (a): one file, read whole, scanned by brute force.** No SQLite, no `rusqlite`, no HNSW.

1. **One file.** The format already exists and is versioned (`bevy_siren index 1`), with a header, a record count and a
   checksum over the body. It becomes the store rather than being replaced by one.
2. **Binary at the scale that matters.** Text is 3× the size of `f32` and 6× `f16` because `{:?}` prints eight or nine
   characters per float. The text form stays as the **debuggable** form — it is how a person reads an index — and the
   binary form is what a 5 000-clip library ships.
3. **`f32`, not `f16`.** `f16` halves the file to 5.2 MB and preserved the top 10 exactly, but that is one query
   against one library, and `rank::top_k` compares scores against a `floor`. A 5 MB saving does not justify a precision
   question at a threshold boundary.
4. **Brute force below 50 000.** Scaling the measured rate linearly puts 50 000 at **~0.5–19 ms**, still inside a
   frame. An ANN becomes worth its dependency above that, and the crate should not pay for it to avoid a cost it does
   not have.

## The key, and the two things that are not the key

**Identity is the content hash** (`library::fnv1a` over the file's bytes, 64-bit). Path is not identity: a rename
changes the path and not the hash, and the index must not notice.

| Event | Hash | Rule |
|---|---|---|
| rename or move | unchanged | **keeps its vector.** The manifest's `paths` is the renameable part |
| re-encode Vorbis → WAV | **changes** | a **new asset**. Same music, different bytes, and the crate cannot tell those apart without content analysis it does not have. A documented non-goal, not a silent merge |
| two files, identical bytes | identical | **one vector, two paths.** `Library::duplicated` already reports these |
| 64-bit collision | equal | **detected, not assumed away.** A hash match whose file size differs is a collision; the index stores `bytes` and refuses the merge rather than dropping an asset |

A 64-bit hash over a few thousand files will not collide by chance, but the check is one comparison and the failure it
prevents is silent.

## The update story

Incremental, as a **set difference** — `#16`'s job, and this ADR's constraint:

- **add**: hash in the fresh scan but not the index → embed, insert.
- **remove**: in the index but not the scan → drop.
- **unchanged**: same hash → **nothing**, not even a re-read.

A whole-file rewrite is acceptable at 5 000 clips (31.9 MB text, 10.3 MB binary) and avoids a journal, shard directory
and partial-write state. Write amplification is the reason SQLite was considered; at this size it is not worth a
dependency.

**Model invalidation is not this ADR's.** `ModelStamp` already refuses an index from a different model, which is the
correct default; a re-embed path is `N-04`.

## The read path

- **Allocation:** read whole into one `Vec<f32>` plus a `Vec<u64>` of hashes; a query allocates one `Vec<f32>` of
  dimension 512 for its own embedding and **nothing per record**. No parsing on the hot path.
- **A query needs no writable filesystem.** Reading is `File::open` + `read_to_end`; nothing is memory-mapped
  `MAP_SHARED`, nothing is locked, nothing is written. A game can ship an index inside its own asset bundle and rank
  from a read-only filesystem — the property #5 asks to have stated.
- **Writing** needs write access and is the editor's operation, not the game's.

## Rejected options

| Option | Why not |
|---|---|
| **(b) SQLite via `rusqlite`** | Pulls a C dependency into a crate whose `ALLOWED_DEPS` a test enforces, to get transactions and an index structure for a 10.3 MB file and a 0.05 ms query. The write amplification it solves is a 10 MB rewrite. |
| **(c) A directory of per-asset records** | Cold load becomes thousands of `open` calls — slower than one file — and a partial write leaves a store with no single checksum to detect it. |
| **HNSW / an ANN crate** | Measured unnecessary at 50 000 and below; adds a dependency and gives up exactness at the `floor` boundary. |
| **`f16`** | Halves the file and preserved one query's top 10, but changes the precision at which a score meets a threshold the crate exposes. Revisit with `V-01`'s labelled set. |

## What this costs

- **A binary format needs a spec**, and a text one does not. The text form stays as the readable form and the binary
  form is additive, distinguished by its magic.
- **No transactions.** A crash mid-write can leave a truncated file; the checksum over the body detects it and the
  failure is a refusal, not a partial index.
- **A whole-file rewrite per incremental update.** At 10.3 MB for 5 000 clips that is milliseconds; at 50 000 it is
  103 MB and becomes the thing to revisit, with the same measurement that would justify an ANN.
