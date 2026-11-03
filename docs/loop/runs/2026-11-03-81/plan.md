# Iteration 81 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **#16 incremental indexing** | p0, unblocked, and the first item in seven iterations that is a build rather than a repair. It is also the one a developer feels: re-embedding a thousand sounds to add one is why people stop running a tool. |
| 2 | #14 licence and source metadata | p0, unblocked, but smaller and less felt. |
| 3 | #21's index round trip | Waiting on a decision file that ADR 0004 now makes answerable. |
| 4 | The binary index writer | ADR 0004's shape, not yet code. |
| 5 | #22 spectral features | Unblocked by ADR 0003. |

**Why (1) beats (2) in one sentence:** #14 records provenance and #16 decides whether the tool is usable at all — and
`stale`, `vanished`, `added_since` and `removed_since` already exist, so this is composition rather than invention.

## The design

**`semantic::incremental::update(root, index, embed) -> (Index, Incremental)`** — a pure function over the index and a
fresh scan. The caller writes with the existing `indexer::write`; this module never becomes a second writer, which is
what the issue's lock warning is about.

| Set | Action | Count |
|---|---|---|
| on disk, not indexed | embed | `added` |
| indexed, not on disk | drop the vector | `removed` |
| both | nothing — not even a re-read | `unchanged` |
| scan refused an audio file | carried as a **named failure** | `failed` |

**No new field on a serde type, no new threshold, no magic constant.** The only number is `EMBEDDING_DIM`, which comes
from the model.

### The two cases the issue calls decisions

- **A rename keeps its vector**, because the index stores hashes and vectors and *no paths* — a move is
  indistinguishable from no move, so it costs nothing **by construction** rather than by a rule someone could break.
- **A re-encode is a new asset.** The bytes changed, so the hash did. The crate cannot distinguish "re-encoded" from "a
  different take" without content analysis it does not have, and treating a genuinely different sound as unchanged
  would leave the index describing audio that is no longer there — the worse error.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/incremental.rs` | **new** — `update`, `Incremental`, `IncrementalProblem`, 7 unit tests |
| `src/semantic/indexer.rs` | `embed_one` becomes `pub(super)` so the add path is **one** code path, not two |
| `src/semantic/mod.rs` | `pub mod incremental;` |
| `tests/semantic_incremental.rs` | **new** — the four acceptance criteria against real WAV files, 6 tests |
| `tests/all.rs` | one `mod` declaration |

## Tests that prove it

Each acceptance criterion is one test, named for the criterion, and each asserts the **embedder's call count** as well
as the index's contents:

1. `adding_one_file_embeds_exactly_one` — 100 files, add one, `calls == 1`.
2. `a_deleted_file_leaves_no_vector` — asserts the **hash** is absent, not that the length changed.
3. `a_rename_costs_nothing` — `calls == 0` and every vector is **identical**, not merely close.
4. `every_run_reports_the_three_counts` — a mixed run, then a re-encode run that must report an add *and* a removal.

## Carved off

- **A filesystem watcher**, which the issue puts explicitly out of scope.
- **Model-version invalidation** — `N-04`; `Index::load` already refuses a foreign stamp.
- **A lock**, because there is no writer here to lock: `indexer::write` owns that, and a caller composes them.
