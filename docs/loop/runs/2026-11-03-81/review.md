# Iteration 81 — review

## Pass 1 — Function: do #16's four criteria hold?

**Critique.** Each criterion is one test, and each asserts something a wrong implementation would fail:

| # | Criterion | Test | Why it cannot pass vacuously |
|---|---|---|---|
| 1 | adding one to a 100-file library embeds **exactly one** | `adding_one_file_embeds_exactly_one` | asserts the **embedder's call count is 1**, so re-embedding everything and discarding extras fails it |
| 2 | a deletion removes its vector | `a_deleted_file_leaves_no_vector` | asserts the **hash is absent**, not that the length fell — which would pass for dropping the wrong vector |
| 3 | a rename keeps its vector | `a_rename_costs_nothing` | asserts `calls == 0` **and** that every vector is identical, not merely close |
| 4 | the three counts are separate | `every_run_reports_the_three_counts` | a mixed run asserts `(1, 1, 1)` exactly |

The re-encode case — the issue's own *"real decision"* — is asserted in test 4 rather than left to prose: a rewritten
file must count as **one add and one removal**, never as unchanged.

**The strongest part of the design.** The index stores `hash → vector` and **no paths**, so a rename is not a case the
code handles — it is indistinguishable from a file that did not move. That is better than a rule correctly implemented:
there is nowhere for it to be wrong.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the failure that happened

**Critique.** The first run of `an_undecodable_file_is_failed_not_counted` **failed**, and it was a real defect:

```
assertion `left == right` failed: and the broken one is a named failure
  left: 0
 right: 1
```

A corrupt `broken.wav` appeared in **no count at all**. `update` iterated `library.assets` and ignored `library.skipped`,
so a file that *is* audio by extension but would not read vanished silently — **the exact silence this crate refuses at
every other layer**, reproduced in the newest code. `indexer::build` already splits those skips by meaning and reports
the decodable-but-broken ones; the incremental path did not.

Fixed by carrying the scan's non-`NotAudio` skips into `Incremental::failed`. The test that caught it asserts a count,
not "something sensible happened", which is why it caught it.

**A second, smaller finding**: `mixed_index(index)` in the test file is an identity function, and it is noise rather than
a seam. It stays because it names the step, but it earns no defence beyond readability.

**Verdict: GREAT.** The defect was found by the iteration's own test and fixed inside it, which is the loop working.

## Pass 3 — Fresh eyes: is it usable without the code explained?

**Critique.** A developer has four numbers and a sentence: `Incremental::describe()` returns
`"1 added, 1 removed, 1 unchanged"`, and `is_noop()` says whether there is anything to write. `update` takes the index
a previous run wrote and returns a new one, so the composition is:

```
let (index, report) = incremental::update(root, old_index, embed)?;
if !report.is_noop() { indexer::write(root, &index)?; }
```

That is the whole workflow, and it needs no knowledge of the internals.

**The one thing a reader might trip on**, and it is stated in the module doc: `Incremental::moved` **does not exist**,
because the index has no paths to compare. An earlier draft of this module had that field and was wrong — it would have
required inventing path data the index does not store. The doc says why the case needs no handling rather than leaving
a reader to wonder where it went.

**Does it hold under a different shape?** The acceptance tests use real WAV files written to a scratch directory, 6
distinct scenarios, with a deterministic content-dependent embedder. That embedder is a stand-in for the model, which
is the honest boundary: this iteration proves the *orchestration*, and the model's own path is covered by iteration
76's cosine measurement.

**Verdict: GREAT.**
