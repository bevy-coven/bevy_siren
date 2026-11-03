# research.md — iteration 73: the indexer, and the three things that were missing

## S1 — what the audit found, and it was worse than a missing feature

Iteration 72's remaining list pointed at `#39`'s encoder as *"genuinely blocked"*. Auditing what actually existed turned
up three gaps, in increasing severity:

| Gap | Evidence |
|---|---|
| the semantic layer is **undocumented** — zero mentions in `README.md`, no page in `docs/guide/` | `grep -ci semantic README.md` → **0** |
| `Model` exposes **`embed_text` only**; there is no audio embedding | `grep "pub fn embed" src/semantic/model.rs` → one hit |
| **nothing builds an index from a folder** | `grep "scan\|build_index\|embed_folder"` over `src/semantic/*.rs` → **nothing** |

The third is the one that made the pick obvious: `Index::new` plus `Index::insert` is not an indexer. A caller who
wanted an index of their library had to write the loop — and get the content hash, the skip list, the failure policy
and the stamp right on their own.

## S2 — the technique: compose the pieces, and make the embedding a seam

[`build`] walks a folder and, for each file the scan found: **decode** (rate from the stream, not the header) →
**resample** (tail flushed, so 1.000 s stays 1.000 s) → **embed** (the caller's function) → **insert** (under the content
hash, so a rename does not change identity).

**The embedding is a `FnMut(&[f32]) -> Result<Vec<f32>, String>`, not a trait.** Audio embedding needs a mel spectrogram,
which needs a filterbank — its own piece of work with its own numbers. A trait would invite a second implementation
nobody asked for; a function is one thing to plug in, and the orchestration is complete and testable today.

## S3 — the failure policy, and the defect it caught

A library is not all-or-nothing: one corrupt file must not stop an index of a thousand. So a file that fails is recorded
in `Built::failed` with its path and the decoder's own words, and the build continues.

**The first version dropped files on the floor.** `library::scan` reports three kinds of skip — `NotAudio`,
`UndecodableHeader`, `Unreadable` — and the indexer filed only the first into `not_audio` and **ignored the other two**.
A WAV with a corrupt header vanished from both lists.

`a_corrupt_file_is_counted_not_swallowed` caught it: the test asserted `failed.len() == 1` and got `0`. The fix splits
the scan's skips by what they mean — *not audio* is a fact about the folder, *would not read* is a failure — and both
now appear.

**That is the same fault as a refusal without a reason, one layer down**, which is why it is worth a section rather than
a line.

## S4 — what an indexer also owes a caller

Two functions that make an incremental rebuild possible, both keyed on the content hash:

| Function | Question |
|---|---|
| `stale(root, index)` | which hashes are on disk and absent from the index |
| `vanished(root, index)` | which indexed hashes are no longer on disk |

`a_rebuild_is_what_an_incremental_index_asks_about` proves the distinction that matters: **a rename is neither**, because
identity is content. Its first version asserted this about a file that had never been indexed, which is why it failed —
the test was wrong, not the code.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| files indexed per build | one record per **distinct content hash** | `a_folder_of_three_files_becomes_three_indexed_records` |
| a 0.5 s 44.1 kHz file | **24000** samples to the embedder | measured, `every_buffer_reaches_the_embedder_at_the_models_rate` |
| skip kinds the scan reports | **3** — `NotAudio`, `UndecodableHeader`, `Unreadable` | `src/library.rs` |
| index file | `<root>/siren-index.txt` | `write` |

No new constants: every number in this module is a caller's or a measured length.

## What good sounds like

For an indexer, "good" is that a person points it at a folder and learns **what happened** in one sentence:

> `1,204 indexed, 12 not audio, 3 failed. First: `boss_theme.ogg`: unreadable: Permission denied`

Counts, the first failure by name and by reason, and the distinction between "not audio" and "would not read" — which
is the difference between a folder that needs tidying and a file that needs fixing.
