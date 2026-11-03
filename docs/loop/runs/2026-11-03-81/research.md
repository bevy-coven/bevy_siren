# Iteration 81 — research: incremental indexing, and what identity costs

The item is [#16](https://github.com/bevy-coven/bevy_siren/issues/16): *"Incremental indexing keyed by content hash."*
p0, unblocked (#11 and #15 both closed), blocks #36.

## Sources

| Source | The number or rule it reports |
|---|---|
| this crate, `src/library.rs::fnv1a` | the content hash is **FNV-1a, 64-bit**, over the file's bytes |
| this crate, `Library::added_since` / `removed_since` | the set difference already exists, over **content hashes**, and `Library` is a `BTreeMap<u64, Asset>` |
| this crate, `Asset::paths` | a `Vec` — one content, many paths; `Library::duplicated()` reports them |
| this crate, `semantic::index::Index` | `records: BTreeMap<u64, Vec<f32>>` — **hash to vector, and no path at all** |
| this crate, `indexer::{write, read, stale, vanished}` | `<root>/siren-index.txt`; `stale` = on disk, not indexed; `vanished` = indexed, not on disk |
| ADR 0004 (`docs/adr/0004-vector-store.md`) | identity is the content hash; rename keeps the vector, re-encode is a new asset; a query needs no writable filesystem |

## The technique, in the sources' terms

An incremental index is a **set difference**, and the crate already has every part of it: a scan keyed by content hash, a
store keyed by content hash, and two functions (`stale`, `vanished`) that answer each side. What was missing is the
**composition** — take an index, ask what changed, embed only the adds, drop only the removals, report the counts.

The decisive observation is that **the index stores no paths**. `Index::records` is `hash → vector`. So a rename is not
a case the code has to handle; it is *indistinguishable from a file that did not move*, and therefore costs nothing by
construction. That is stronger than a rule someone remembered to implement: there is no place for the rule to be wrong.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| identity | content hash, 64-bit FNV-1a | `library.rs` |
| the three sets | on disk only → add; indexed only → remove; both → unchanged | the issue's own scope |
| rename | **unchanged** — the hash did not move | ADR 0004; measured by `a_rename_costs_nothing` |
| re-encode | **a new asset** — the bytes moved | ADR 0004; measured by `every_run_reports_the_three_counts` |
| the four counts | added, removed, unchanged, failed | the issue's *"reports adds, removals and unchanged as separate numbers"* |

## What good sounds like

Adding one file to a hundred embeds **exactly one**, and the count is taken from the embedder's own call count so an
implementation that re-embeds everything and discards the extras cannot pass. A rename embeds **zero**. A deletion
embeds **zero** and leaves **no vector**, asserted by looking for that hash rather than by a count that would pass for
dropping the wrong one. And a file that will not embed is a **named failure**, never silently absent from all three
counts — which is the bug this iteration's own test caught in its first draft.
