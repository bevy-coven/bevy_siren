# plan.md — iteration 71: the ensemble's breakdown (`#40`'s structure)

## The pick, and why it beats the runner-up

**Pick:** `#40`'s **structure** — `Signal`, `Contribution`, `Breakdown` and the arithmetic that makes a score add up —
in `src/model.rs` where the engine-free scan covers it.

**Runner-up:** nothing on the list is closer to buildable. Three of the five remaining items are decisions
(schema, labelled data, a licence audit) and the fourth is the tool host, which would expose a pipeline whose scores
cannot yet be explained.

**One sentence:** an ensemble that cannot be explained is indistinguishable from a magic number, and the structure that
makes it explainable needs no signal — so it can be built and proved before the encoder that will feed it exists.

## The change

| Item | Where | What it is |
|---|---|---|
| `Signal` | `src/model.rs` | which signal, by name, in a fixed order |
| `Contribution` | `src/model.rs` | raw value, weight, product, and whether the signal **had a value at all** |
| `Breakdown` | `src/model.rs` | four contributions; `score`, `shares`, `strongest`, `explain` |
| the **enforcement** | `tests/all.rs` | `the_ensemble_lives_in_the_engine_free_model`, so moving it out fails |

## Every number, and its source

No new constants. The arithmetic is a **sum**; every number is a caller's. The one authored decision is sum rather than
mean, and the argument is in the source: *the weights are the normalisation*, so dividing by the total would undo a
project's statement that a signal matters half as much.

## The tests that prove it

| Behaviour | Test |
|---|---|
| the score is the sum of contributions | `the_score_is_the_sum_of_every_contribution` — **0.64** by hand |
| each part keeps raw, weight and product | `each_part_keeps_its_raw_value_its_weight_and_its_contribution` |
| a missing signal is not a zero | `a_missing_signal_is_not_a_zero_and_says_so` |
| a NaN contributes nothing | `a_non_finite_value_contributes_nothing_rather_than_poisoning_the_sum` |
| a negative weight contributes nothing | `a_negative_weight_contributes_nothing_rather_than_repelling` |
| shares sum to 1 | `the_shares_sum_to_one_and_say_what_the_result_was_made_of` |
| a zero score has no shares | `a_zero_score_has_no_shares_rather_than_four_nans` |
| a penalty survives as the signal against | `a_penalty_is_kept_as_the_signal_that_counted_against` |
| the sentence carries the numbers | `the_explanation_names_the_raw_value_and_the_weight` |
| nothing contributed says so | `a_breakdown_with_nothing_in_it_says_so_rather_than_naming_a_reason` |
| ties are decided, not lucky | `ties_go_to_the_earlier_signal_so_two_runs_agree` |
| names match a project's weights file | `the_signal_names_are_the_field_names_a_project_writes` |
| **the arithmetic stays engine-free** | `engine_free::the_ensemble_lives_in_the_engine_free_model` |

## What was carved off, and it is the issue's own caveat

`#40` warns that *"a raw cosine in [−1, 1] and a loudness penalty in LU must be mapped onto a common range before
weighting, and that mapping is itself an authored decision."*

**This iteration does not solve that**, and says so in `research.md` S4: the mapping is a property of the signals, and
there are no signals without an encoder. What it does is make the problem **visible** — a `Contribution` reports its
`raw` value beside its `weight`, so a caller can see that one signal arrives in `[−1, 1]` and another in decibels. A
breakdown showing only products would hide exactly that.

Also carved off: `#39`'s encoder, `V-01`'s data, and where the weights live — the first two are blocked and the third is
a decision.
