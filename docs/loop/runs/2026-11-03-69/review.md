# review.md — iteration 69, three passes

## Pass 1 — Function: does the metric measure what it claims?

| Claim | Test | Result |
|---|---|---|
| recall is a **prefix** test | `recall_uses_a_prefix_not_a_membership_test` | rank 9 is not found by k = 3 |
| agreement counts affirmed neighbours above the anchor | `agreement_is_the_share_of_affirmed_neighbours_above_the_anchor` | 2 of 3 = 0.667 |
| an absent anchor scores zero | `an_anchor_absent_from_its_own_ranking_scores_zero` | 0.0 |
| an absent neighbour is a miss, not a "below" | `a_neighbour_absent_from_the_ranking_counts_as_a_miss` | 1 of 2 |
| a miss is named **with its rank** | `a_miss_names_the_description_and_its_actual_rank` | *"`white noise hiss` (rank 2)"* |
| an uncovered description reads as **absent** | `a_description_with_no_ranking_is_a_miss_named_absent` | *"(absent)"* |
| an empty set is not perfect | `an_empty_set_is_not_perfect` | `perfect()` false |

**And the measurement**, on iteration 68's real cosines:

| floor | recall@1 |
|---|---|
| 0.000 | **100 %** |
| 0.126 | **100 %** |
| 0.150 | 75 % |
| 0.500 | **0 %** |

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Two clippy defects, both in the new test:

| Defect | Found by | Fix |
|---|---|---|
| a **very complex type** for the measured rows | clippy | a named `type Measured` |
| `assert!` on two constants is a **constant assertion** | clippy | moved into `const { … }`, where it stops the build instead of failing a test |

And one **in my own working**, recorded because it was destructive: `git checkout -- .` to free space **reverted the two
tracked edits** this iteration had made. The new files survived because they were untracked. Both were restored and
verified with `grep`. The narrower command — `git checkout -- examples/evalprobe.rs` — is what was wanted.

The metric's own awkward cases are covered deliberately: a prefix rather than a membership test, an absent anchor, an
absent neighbour, an empty judgement list, and a description the run never covered. Each of those is a way a naive
implementation reports a higher number than it earned.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **`describe()` names the failures**: *"recall@k 50% over 2 descriptions, agreement 100% over 0 anchors. Missed:
  `white noise hiss` (rank 2)."* A count alone would be a scoreboard, not a regression report.
- **It owns nothing**: `score` takes rankings a caller supplies, so its tests run in **0.00 s** and the real harness can
  feed it anything without the metric knowing what produced it.
- **`at_floor` is the whole point**: it answers *"if the floor were this, how many right answers survive?"*, which is
  exactly the question `V-01` exists to answer with evidence.
- **The default build is untouched**: 181 tests with no features; the module is behind `semantic`.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | the evaluation metric — `#45`'s metric half, `V-01`'s consumer |
| new module | `src/semantic/eval.rs` |
| tests | **+15**, from 309 to **324** all-features |
| measured | floor 0.0 → **100 %** recall; 0.126 → **100 %**; 0.150 → **75 %**; 0.5 → **0 %** |
