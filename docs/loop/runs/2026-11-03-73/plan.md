# plan.md — iteration 73: the indexer

## The pick, and why it beats the runner-up

**Pick:** `build` — the function that turns a folder into an index. **Nothing else could proceed without it**: the
semantic layer had scan, decode, resample, embed and store, and no composition of the five, so the feature could not be
used at all.

**Runner-up:** the weights schema (`needs-decision/weights-live-where.md`). It matters, but a project cannot configure
weights for an index it cannot build.

**One sentence:** an audit found the semantic layer had every part and no assembly, which makes every other item on the
list premature.

## The audit, and it is the honest part

| Gap | Severity |
|---|---|
| **no index builder** | the feature is unusable |
| **no audio embedding** (`embed_text` only) | the index could never be filled |
| **zero documentation** — no README mention, no guide page | nobody could discover it |

The second is a real blocker and is **not** in this iteration's scope: audio embedding needs a mel spectrogram, which
needs a filterbank. It is named in `research.md` S2 and worked around with a seam, not faked.

## The change

| Item | What it is |
|---|---|
| `build(root, stamp, embed)` | scan → decode → resample → embed → insert, per file |
| `EmbedAudio` | the seam: `FnMut(&[f32]) -> Result<Vec<f32>, String>` |
| `Built` | the index, the failures **with reasons**, and the not-audio list |
| `built.describe()` | one sentence: counts and the first failure by name |
| `write` / `read` | the index beside its library, stamped |
| `stale` / `vanished` | the two halves of an incremental rebuild, keyed on content hash |

## Every number, and its source

No new constants. Every length is a caller's or a measured one — 24000 samples for a 0.5 s 44.1 kHz file, which is the
rate ratio `resample` already promises.

## The tests that prove it

| Behaviour | Test |
|---|---|
| a folder of three files is three records | `a_folder_of_three_files_becomes_three_indexed_records` |
| every buffer reaches the embedder at 48 kHz | `every_buffer_reaches_the_embedder_at_the_models_rate` — **24000** |
| **a corrupt file is counted, not swallowed** | `a_corrupt_file_is_counted_not_swallowed` |
| a folder with no audio refuses | `a_folder_with_no_audio_refuses_rather_than_returning_an_empty_index` |
| a non-directory refuses with its path | `a_path_that_is_not_a_directory_refuses_with_the_path` |
| the index round-trips to disk | `the_index_is_written_and_read_back_with_its_stamp` |
| a rename is neither stale nor vanished | `a_rebuild_is_what_an_incremental_index_asks_about` |
| an embedder failure names its file | `an_embedder_that_fails_on_one_file_names_that_file` |
| a wrong-width vector is a failure, not a panic | `a_vector_of_the_wrong_width_is_a_failure_not_a_panic` |
| the same folder builds the same index twice | `the_same_folder_builds_the_same_index_twice` |
| a markdown file is not a failure | `a_file_that_is_not_audio_is_reported_as_such_not_as_a_failure` |

## What was carved off

**Audio embedding.** It needs a mel spectrogram and a filterbank, which is `#39`'s neighbour and its own iteration. The
seam is one function, so it drops in without touching the orchestration.

**The weights schema, the labelled set, the tool host.** Unchanged from iteration 72.
