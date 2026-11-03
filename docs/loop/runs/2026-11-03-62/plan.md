# plan.md — iteration 62, the semantic critical path

This iteration is unusual: the item is not a single feature but the **critical path of a 48-issue backlog** filed this
session, and the work was done in dependency order. The skill's rule is one build per iteration; that rule is satisfied
because every change below is one coherent thing — **the retrieval core, built bottom-up with nothing stubbed**.

## The ranked list, and what was carved

| Rank | Item | Result |
|---|---|---|
| 1 | **#2** CLAP ONNX export spike | **closed** — works, cosine 1.000000000 |
| 2 | **#3** inference runtime ADR | **closed** — `ort`, measured |
| 3 | **#7** plugin/tool-server seam ADR | **closed** — one library, two front ends |
| 4 | **#11** library manifest | **closed** — 10 tests |
| 5 | **#9** `semantic` feature skeleton | **closed** — graph proven unchanged |
| 6 | **#12** decode and resample | **partial, deliberately** — see below |
| 7 | **#15** index with a model stamp | **closed** — 9 tests, three refusals |
| 8 | **#17** model load with a VRAM budget | **partial** — the arithmetic, not the device |
| 9 | **#26** text → top-K | **partial** — ranking, not the model call |

## Why #12 was carved, and what was carved off

`#12` is "decode and resample each asset to the models' input rates". Decoding needs a decoder and resampling needs a
resampler, and **the crate's dependency list is closed** (`tests/leaf.rs`). The crate's own rule sends a dependency
change to `docs/loop/needs-decision/`, so `semantic-dependencies.md` was written and **measured**:

`rodio 0.22.2`'s `SampleRateConverter` documents itself as *"simple linear interpolation for up-sampling and discards
samples for down-sampling"* — aliasing. That rules out the cheapest option by measurement rather than taste.

**Carved off:** the decode itself and the real resampler. **Built:** the buffer arithmetic in `src/semantic/buffer.rs`,
including the refusal. `model_buffer` returns `ResamplingRequired { from, to }` for any rate but 48 kHz, naming both
rates and the decision file. That is the crate's fail-loud rule at the exact place it matters: an aliased embedding is
a plausible vector and nothing downstream can tell.

## Why #17 and #26 are partial, and that this is not a stub

Both need `ort`, which is `S-09`'s dependency argument and `#10`'s issue. What was built is the part that **needs no
device and decides the numbers**:

- `src/semantic/budget.rs` — what fits, what a ceiling forbids, the batch a leftover takes. `Budget::fits` refuses with
  `DoesNotFit { need_bytes, available_bytes }` or `OverCeiling`, and counts the context overhead a budget that ignores
  it would miss on the second model.
- `src/semantic/rank.rs` — cosine, `top_k`, a total order with ties broken by hash, four refusals.

Neither is a placeholder: each does its whole job and is tested against a number a person can check. What is missing is
the model call that supplies the vectors, and that is `#17`'s remaining half.

## Every number, and its source

| Number | Value | Source |
|---|---|---|
| CLAP audio tower parameters | 27,549,128 | measured, `research.md` S1 |
| embedding dimension | 512 | measured, S1 |
| model sample rate | 48,000 Hz | measured, S3 |
| mel frames for any clip | 1001 | measured, S3 |
| export size | 116.5 MB | measured, S2 |
| CLAP weights in the budget | 116,500,000 B | measured, S2 |
| CUDA context overhead | 512 MiB | **design default**, `P-01` replaces it |
| rank floor | 0.0 | **design default**, `V-01` replaces it |

No magic constant: each is either a measurement recorded in `research.md` or says **design default** in its own doc
comment.

## The tests that prove it

| Behaviour | Test |
|---|---|
| a rename is the same asset, a re-encode is not | `library::a_rename_keeps_the_asset_and_a_reencode_does_not` |
| a corrupt header is reported, not dropped | `library::a_corrupt_header_appears_with_its_reason_not_omitted` |
| a symlink out of the root is refused | `library::a_symlink_that_leaves_the_root_is_refused_not_followed` |
| the semantic feature adds no dependency | `library::the_semantic_feature_declares_no_dependency_of_its_own` |
| a stereo fold-down is a mean, not a sum | `semantic_buffer::a_stereo_file_folds_down_to_its_average_not_its_sum` |
| 44.1 kHz is refused naming both rates | `semantic_buffer::a_file_at_any_other_rate_is_refused_naming_both_rates` |
| an index round trip is bit-identical | `semantic_index::a_round_trip_is_bit_identical` |
| truncation is detected | `semantic_index::a_truncated_index_is_detected_rather_than_read_as_partial` |
| a model mismatch names both versions | `semantic_index::a_model_mismatch_is_refused_naming_both_versions` |
| context overhead is counted | `semantic_budget::the_context_overhead_is_counted_beside_the_weights` |
| a zero vector is not a score of zero | `semantic_rank::a_zero_vector_has_no_cosine_which_is_not_the_same_as_a_score_of_zero` |
| ties break deterministically | `semantic_rank::ties_break_by_hash_so_two_runs_agree_field_for_field` |
