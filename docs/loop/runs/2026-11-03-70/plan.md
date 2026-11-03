# plan.md — iteration 70: the ensemble weights (`#41`)

## The pick, and why it beats the runner-up

**Pick:** `#41`, the ensemble weights as a serde type. It is item 3 of iteration 69's list and the first item on it that
is **decidable today**: `V-01` needs a person and a licence-cleared corpus, `#45`'s runner needs `V-01`'s data, and
`#28` exposes a pipeline whose thresholds are still unmeasured.

**Runner-up:** `#28`, the MCP tool host. It loses on order: a tool surface built before the weights exist would have to
invent its defaults at a call site, and then keep them there — which is the thing `#41` exists to prevent.

**One sentence:** the weights type is where a project's own opinion about "fits" becomes data, and building it before
the ensemble means the ensemble has somewhere to put its numbers rather than inventing them.

## A dependency the issue states that the code does not

`#41` says **Blocked by #40**. But `#40` needs `#39`'s music-specific encoder, which is blocked on `S-05`'s licence audit
— a decision. The weights type is what `#40` would *consume*, so building it first inverts nothing and unblocks the
half that can be judged.

## The change

`src/semantic/weights.rs`:

| Item | What it is |
|---|---|
| `Weights` | six fields: four ranking weights, a loudness tolerance, a drop fraction |
| `Weights::problems()` | every problem, not just the first — the crate's `CueTable::problems` pattern |
| `WeightProblem` | five variants, each naming the field, the value and the fix |
| `Weights::shares()` | each weight's fraction of the total, for a result's breakdown |
| `from_ron` / `to_ron` | behind `serde`, the crate's existing RON path |

## Every field, its default and its source

| Field | Default | Source |
|---|---|---|
| `semantic` | **1.0** | **design default** — the unit the others are relative to |
| `acoustic` | **0.5** | **design default**, argued from the measured band overlap (0.126–0.429 vs −0.104–0.150) |
| `tempo` | **0.35** | **design default** — below `acoustic`, because a tempo is absent for every effect |
| `key` | **0.35** | **design default** — equal to `tempo`, because nothing measures which matters more |
| `loudness_tolerance_db` | **6.0** | **design default** — below the crate's own 8 dB duck, so a deliberate level difference is not penalised |
| `drop_fraction` | **0.0** | **design default**, from iteration 69's recall table (100 % at 0.126, 75 % at 0.150, 0 % at 0.5) |

No magic constant: each is a named field with the argument for its value in its own doc comment, and the two that rest
on this crate's measurements say so.

## The tests that prove it

| Behaviour | Test |
|---|---|
| the defaults are the argued ones | `the_defaults_are_the_ones_the_docs_argue_for` |
| the default has nothing wrong with it | `the_default_configuration_has_nothing_wrong_with_it` |
| all-zero is refused with a fix | `all_zero_weights_are_refused_with_a_message_that_says_what_to_do` |
| a negative weight names the field | `a_negative_weight_names_the_field_and_the_sign_error` |
| a NaN is refused | `a_non_finite_weight_is_refused_rather_than_compared_false` |
| a zero tolerance is refused | `a_tolerance_of_zero_or_less_is_refused` |
| a bad drop fraction is refused | `a_drop_fraction_outside_zero_to_one_is_refused` |
| **an unusual choice is allowed through** | `an_unusual_but_possible_configuration_is_allowed_through` |
| every problem is reported, not the first | `every_problem_is_reported_not_just_the_first` |
| shares sum to 1 | `the_shares_sum_to_one_and_say_what_a_result_was_made_of` |
| zero weights have no shares, not NaNs | `all_zero_weights_have_no_shares_rather_than_four_nans` |
| the RON round trip | `the_weights_round_trip_through_ron` |
| an old file gains defaults | `a_file_written_before_a_field_existed_still_loads_with_that_fields_default` |
| a typo is refused | `a_misspelled_field_is_refused_rather_than_silently_ignored` |
| a parseable file can still mean nothing | `a_file_that_parses_but_means_nothing_is_caught_by_problems_not_by_the_parser` |

## What was carved off

**Nesting into `SirenSettings`.** The issue's notes say to nest rather than add a parallel root, and that is right — but
`SirenSettings` is a public type whose file the Cauldron already reads, so adding a field to it changes the definitions
file's schema. **That goes to `needs-decision/`**, and this iteration ships the type and its report without deciding
where a project writes it.

**`#40`, `#39`, `V-01`, `#45`'s runner, `#28`** — all still open.
