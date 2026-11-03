# review.md — iteration 62, three passes

## Pass 1 — Function: do the pieces do what their sources say?

**#2's spike.** The question was whether CLAP's HTSAT audio tower exports to ONNX. Measured: **yes**, at
**cosine 1.000000000** and **6.8 × 10⁻⁷** max absolute difference, on 1 s, 4 s and 10 s clips. 39 standard operators,
`onnx.checker` passing, opset 17.

Two findings beyond the question, both of which changed other issues:

1. **The time axis is not variable.** Every duration yields 1001 mel frames. `I-03`'s chunker is a counting problem,
   not a shaping problem.
2. **Dynamic batch is broken** at batch ≥ 2, and a fixed-batch export is exact. `N-02` must not assume one graph
   accepts any batch.

**The library manifest.** `a_rescan_of_an_unchanged_directory_is_identical` passes, so a scan is a pure function of the
tree. `a_rename_keeps_the_asset_and_a_reencode_does_not` reports **0 adds / 0 removals** for a rename and **1 / 1**
for a re-encode in place — the distinction the incremental index exists for.

**The index.** `a_round_trip_is_bit_identical` asserts exact equality, made honest by
`a_float_round_trips_through_its_shortest_form`, which pins the `{:?}`-parses-back claim on `f32::MIN_POSITIVE`,
`MAX`, `1/3` and `-1.2345679e-7`. Three refusals fire: mismatch, truncation, checksum.

**Ranking.** `a_cosine_a_person_can_check_by_hand` — (3,4) vs (4,3) is **0.96**, which is `24/25` and verifiable
without trusting the code.

**Verdict: GREAT.** Every claim is traceable to a measurement or a hand-checkable number.

## Pass 2 — Zoom: read the awkward cases

Five defects were found **by the tests themselves**, which is the point of writing them against a number:

| Defect | Found by | Fix |
|---|---|---|
| A RIFF/WAVE with no `fmt ` chunk was **indexed** with unknown fields | `a_corrupt_header_appears_with_its_reason_not_omitted` | the chunk walk now returns `None`, so it is reported corrupt |
| My re-encode fixture left **two** assets | `a_rename_keeps_the_asset_and_a_reencode_does_not` | the test's expectation was wrong; an in-place re-encode is one file, 1 add and 1 removal |
| `model_buffer` would have **silently converted** 44.1 → 48 kHz | the decision file's own argument, read back | removed the interpolator entirely; the rate mismatch is now a refusal |
| `IndexProblem` was large enough to **pad every `Result`**, including successes | clippy | boxed the two-stamp variant |
| The GPU ceiling was reported as **device space** | `a_ceiling_is_reported_as_the_projects_own_setting_not_the_cards` | `device_free_bytes` and `available_bytes` are separate; the message names the setting the reader can change |

The third is the one that mattered most. Shipping an interpolator while the decision file recommended against it would
have been **two answers to one question**, and the interpolation would have been the answer that ran.

**Verdict: GREAT.** Each defect is named, fixed, and covered by the test that found it.

## Pass 3 — Fresh eyes: does it hold without me?

The question for this iteration is not a seed or a cue set — no audio path changed. It is whether **the pieces hold
when recombined**, and whether a developer can use them without the code.

- **Recombination.** An index made by `Index::insert` from vectors, written with `to_text`, read back with `load`,
  queried with `top_k` — three modules tested separately and one path through them. The round trip is bit-identical, so
  a vector that survives the file is the same vector `top_k` scores.
- **Without the code.** Every refusal's `Display` was read, not just its variant:
  - *"this index was built by … and … is loaded. Re-index the library, or set the model version to the index's."*
  - *"the file is 44100 Hz and the model needs 48000 Hz; no resampler is approved yet
    (docs/loop/needs-decision/semantic-dependencies.md)"*
  - *"this would commit 1250 MB of a 1024 MB ceiling this project set. Raise the ceiling, or load fewer models."*

  Each names what was found, what was wanted, and what to do. None says "invalid input".
- **The default build is untouched**, which is the claim a 0.1.0 crate must be able to make: 413 dependency lines
  either way, 174 tests passing with no features.

**Verdict: GREAT.**

## The whole run

| | |
|---|---|
| issues closed | **6** — #2, #3, #7, #9, #11, #15 |
| issues advanced | **3** — #12, #17, #26 (the device-free half of each) |
| new code | `library.rs`, `semantic/{mod,buffer,index,budget,rank}.rs` |
| tests | **+44**, from 174 to 218 with no features / 237 all-features |
| new decisions | 2 ADRs, 1 needs-decision file |
