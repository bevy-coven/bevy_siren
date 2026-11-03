# review.md — iteration 73, three passes

## Pass 1 — Function: does a folder become an index?

| Claim | Test | Result |
|---|---|---|
| three files → three records | `a_folder_of_three_files_becomes_three_indexed_records` | 3 indexed, 0 failed |
| every buffer at the model's rate | `every_buffer_reaches_the_embedder_at_the_models_rate` | **24000** for 0.5 s at 44.1 kHz |
| a corrupt file is **counted** | `a_corrupt_file_is_counted_not_swallowed` | 1 indexed, **1 failed**, reason a sentence |
| no audio → refuse | `a_folder_with_no_audio_refuses_rather_than_returning_an_empty_index` | `NothingFound` with the path |
| not a directory → refuse | `a_path_that_is_not_a_directory_refuses_with_the_path` | `NotADirectory` |
| the index survives the process | `the_index_is_written_and_read_back_with_its_stamp` | 2 records, stamp intact |
| a rename is neither stale nor vanished | `a_rebuild_is_what_an_incremental_index_asks_about` | both counts as asserted |
| an embedder failure names its file | `an_embedder_that_fails_on_one_file_names_that_file` | 1 indexed, 1 failed, the message verbatim |

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Two defects, and the first is the important one:

| Defect | Found by | What it was |
|---|---|---|
| **the scan's failures vanished** — `UndecodableHeader` and `Unreadable` went into neither list | `a_corrupt_file_is_counted_not_swallowed`, asserting `failed.len() == 1` and getting **0** | a corrupt WAV disappeared from both `failed` and `not_audio` |
| my rename assertion used a file that had **never been indexed** | the same test, on `stale().is_empty()` | the test was wrong, not the code |

The first is **the same fault as a refusal without a reason, one layer down**: a file that would not read was silently
absent from every report. `library::scan` reports three kinds of skip and the indexer filed only one of them. The fix
splits them by what they mean to a caller — *not audio* is a fact about the folder, *would not read* is a failure — and
both now appear.

The second is a reminder that a test asserting an absence (`is_empty`) can be satisfied by never doing the thing.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **One sentence says what happened**: *"1,204 indexed, 12 not audio, 3 failed. First: `boss_theme.ogg`: unreadable:
  Permission denied"* — counts, the first failure by name, and the distinction between a folder that needs tidying and
  a file that needs fixing.
- **The seam is one function**, so the mel path drops in without touching the orchestration — and the module doc says
  plainly that audio embedding is not in the crate yet, rather than implying it is.
- **The default build is untouched**: 198 tests with no features; everything here is behind `semantic`.

**Verdict: GREAT** on the orchestration. **The feature as a whole is not yet usable** — there is no audio embedding and
no documentation — and both are named in `research.md` S1 rather than left for someone to discover.

## The run

| | |
|---|---|
| item | the indexer — the composition that made the feature reachable |
| new module | `src/semantic/indexer.rs` |
| tests | **+11**, from 356 to **367** all-features |
| suites | 198 default / 367 all-features |
| carved off | audio embedding (needs a mel spectrogram + filterbank) |

## What the audit changed about the plan

Iteration 72's list said `#39`'s encoder was *"genuinely blocked on a licence audit"*. That is true of the **music
encoder**; it is not true of **audio embedding for CLAP**, which needs a mel spectrogram and no model choice at all.
The audit found the semantic layer had every part and no assembly, which made every other item on the list premature —
and that is the item that shipped.
