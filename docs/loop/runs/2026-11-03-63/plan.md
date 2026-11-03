# plan.md — iteration 63: the last three decision points, and the pipeline

## The ranked list

Iteration 62 closed six issues on the semantic critical path and left four open: #10, #12, #17, #26. This iteration
finished **three of the four** and advanced the other with a measurement.

| Rank | Item | Result |
|---|---|---|
| 1 | **#10** extend `tests/leaf.rs` — the gate on `ort` | **closed** — measured before it was decided |
| 2 | **#17** the model load | **closed** — `model.rs` proven against a real graph |
| 3 | **#26** text → top-K | **closed** — pipeline built, end to end measured |
| 4 | **#12** decode and resample | **advanced** — the resampler measured, the decision narrowed to one dependency |

## Why this order, and why it is not arbitrary

`#10` gates everything: without `ort` in the graph, `#17` cannot load a model and `#26` cannot encode a query. It was
also the item most likely to be decided by opinion rather than measurement, so it was done first and measured.

## Every number, and its source

| Number | Value | Source |
|---|---|---|
| text tower parameters | 124,645,632 | measured, `research.md` S1 |
| text ONNX size | 501.4 MB | measured, S1 |
| text embedding dimension | 512, L2-normalised | measured, S1 |
| token axis | dynamic, 4 to 15 tokens proven | measured, S2 |
| correct-match cosine band | **0.126 – 0.429** | measured, S3 |
| incorrect-match band | **−0.104 – +0.150** | measured, S3 |
| query latency (M4 CPU) | **5.28 ms** | measured, S4 |
| `ort` build time | **3.2 s** (8 s with the runtime fetch) | measured, `evidence/ort-probe.txt` |
| `ort` features needed | **2** — `download-binaries` + a TLS feature | measured, same |
| `rubato` 1 kHz energy ratio | **0.9763** | measured, `evidence/resampler-probe.txt` |
| `rubato` 15 kHz energy | 0.488094 of 0.500000 | measured, same |
| end-to-end hit rate | **3 / 4**, the miss a 0.006 tie | measured, `evidence/e2e.txt` |

No magic constant. Every one is either a measurement recorded in this run's evidence or says **design default** in its
own doc comment (the CUDA context overhead and the rank floor are the two).

## The tests that prove it

| Behaviour | Test |
|---|---|
| the feature names exactly one dependency | `library::the_semantic_feature_declares_no_dependency_of_its_own` |
| the dependency list is five and each is argued | `leaf::the_dependency_list_stays_closed` |
| a graph's shape is read from the file | `semantic_model::a_graphs_own_shape_is_read_from_the_file` |
| a real graph runs and normalises | `semantic_model::a_graph_runs_and_returns_unit_norm_values` |
| a missing file names the path | `semantic_model::a_missing_file_says_which_file` |
| a stamp mismatch is refused | `semantic_search::a_search_against_a_different_models_index_is_refused` |
| floor drops are **counted** | `semantic_search::the_floor_drops_are_counted_rather_than_silent` |
| a fixed-batch graph admits one size | `semantic_session::a_fixed_batch_graph_admits_only_batch_one` |
| a dynamic-batch graph admits a range | `semantic_session::a_dynamic_batch_graph_admits_a_range` |
| the floor's default is calibrated | `semantic_query::the_measured_bands_overlap_which_is_why_the_default_floor_keeps_everything` |

## What was carved off, and it is stated rather than hidden

- **#17's CUDA half.** The probe ran `CPUExecutionProvider`. A CUDA execution provider needs an ONNX Runtime built
  against CUDA, which is a different artefact. That needs `big`.
- **#17's weight download.** `Model::load` takes a path that exists; a pinned, checksummed fetch is `S-05`'s issue.
- **#26's tokeniser.** `Query` validates text and `Model::embed_text` takes ids and a mask; the mapping between them is
  not in the crate, and adding a tokeniser is its own dependency argument.
