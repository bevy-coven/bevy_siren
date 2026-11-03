# plan.md — iteration 69: the evaluation harness (`#45`'s metric half)

## The pick, and why it beats the runner-up

**Pick:** the metric `V-01` and `#45` need. Every threshold on the semantic path is a **design default** because there
is no labelled set; `V-01` produces that set, and `#45` runs against it. Neither can exist usefully without the metric,
and the metric is decidable **now** while the data is not.

**Runner-up:** `#41` (tunable weights). It is the same kind of item one step earlier — but a weights type without a
metric cannot be evaluated, so building it first would produce a knob nobody can turn responsibly.

**One sentence:** a labelled set with no metric is a data file, and the metric is what turns every design default on
this path into something a test can regress.

## The change

`src/semantic/eval.rs`:

| Item | What it is |
|---|---|
| `Labelled` | a description and the asset a person says it should find |
| `Judged` | an anchor and the neighbours a person **affirmed** fit it |
| `Ranking` | the hashes a run produced, best first |
| `recall_at_k` | found within `k`, as a prefix test |
| `agreement` | the share of affirmed neighbours placed above the anchor |
| `score` | the whole set → a `Report` naming its failures |
| `at_floor` | **what a floor would do to recall** — `V-01`'s whole purpose in one function |

## Every number, and its source

No new constants. The module adds **no** rate, curve or threshold: it measures ones that already exist. The four
figures quoted in its tests are iteration 68's measured cosines.

## The tests that prove it

| Behaviour | Test |
|---|---|
| recall is a prefix test, not membership | `recall_uses_a_prefix_not_a_membership_test` |
| agreement counts affirmed neighbours above the anchor | `agreement_is_the_share_of_affirmed_neighbours_above_the_anchor` |
| an absent anchor scores zero | `an_anchor_absent_from_its_own_ranking_scores_zero` |
| an absent neighbour counts as a miss | `a_neighbour_absent_from_the_ranking_counts_as_a_miss` |
| no judgements is not a failure | `an_anchor_with_no_judgements_is_not_a_failure` |
| a miss is **named with its rank** | `a_miss_names_the_description_and_its_actual_rank` |
| an uncovered description reads as *absent* | `a_description_with_no_ranking_is_a_miss_named_absent` |
| an empty set is not perfect | `an_empty_set_is_not_perfect` |
| a floor that hides the answer turns a hit into a miss | `a_floor_that_hides_the_answer_turns_a_hit_into_a_miss` |
| **the floor table on the real model's cosines** | `a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model` |

## What was carved off

**`V-01` itself** — the labelled assets and human judgements — which needs a person and a licence-cleared corpus, not a
loop iteration. What ships is the thing that will measure it.

**`#45`'s runner** — the binary or example that feeds real rankings into `score`. It needs `V-01`'s data to be worth
writing; the metric is testable without it, which is where the line was drawn.
