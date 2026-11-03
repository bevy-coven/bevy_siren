# review.md — iteration 71, three passes

## Pass 1 — Function: does the score add up, and does the breakdown explain it?

| Acceptance criterion (`#40`) | Test | Result |
|---|---|---|
| the scoring arithmetic lives where no engine type appears, and the scan still passes | `engine_free::model_names_no_engine_type` + `the_ensemble_lives_in_the_engine_free_model` | both green |
| a result reports every component with its weight and contribution | `each_part_keeps_its_raw_value_its_weight_and_its_contribution` | raw, weight, product, all kept |
| the contributions **sum to the score** | `the_score_is_the_sum_of_every_contribution` | **0.64** by hand |
| two calls return identical order and scores, field for field | `ties_go_to_the_earlier_signal_so_two_runs_agree` | the input order cannot change the answer |
| removing one signal changes the order where it matters | *arrives with the ensemble* | **carved off, stated** |

The last is the criterion I did not meet: it needs signals to remove, and signals need `#39`'s encoder. Named in
`plan.md` rather than quietly skipped.

**Verdict: GREAT** on what it claims.

## Pass 2 — Zoom: read the awkward cases

Five behaviours a naive ensemble gets wrong, each with a test:

| Case | What would go wrong | What the code does |
|---|---|---|
| a **tempo never estimated** | reads as "tempo disagreed" | `missing: true`, contributes 0, named in the sentence |
| a **NaN** raw value | poisons the sum, every comparison false | marked missing, scores 0 |
| a **negative weight** | a signal that should attract repels | marked missing, scores 0 |
| a **zero score** | four divisions by zero → four NaNs | `shares()` returns `None` |
| **equal contributions** | the winner depends on iteration order | ties resolved by `Signals` order, asserted both ways |

And one defect **in my own first draft**, caught by the compiler: `strongest()` used `std::ptr::eq` to recover which
signal a contribution belonged to. That is a pointer comparison standing in for a field — it only worked by accident of
array layout. The fix was to give `Contribution` a `signal` field, which also made `Breakdown::new` able to place parts
by their own name so the array order cannot be got wrong.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer read it without me?

The sentence, which is the whole of `#44`'s `explain_match` in a string:

> *"semantic carried it (+0.400 from 0.4 raw at weight 1); acoustic counted against it (-0.100 from -0.2 raw); no value
> for tempo, key"*

It names what carried the result, what counted against it, the **raw values and weights** behind each, and which signals
had nothing to say. A person who has never seen the code can read that and know why a track is first.

- **The names match the weights file**: `Signals.map(Signal::name)` is `["semantic", "acoustic", "tempo", "key"]`, the
  same words `Weights` uses — so a breakdown can be cross-referenced against a project's configuration.
- **The default build is untouched**: 194 tests with no features; the types are in `model.rs`, which has no dependency
  on `semantic`.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | `#40`'s structure: `Signal`, `Contribution`, `Breakdown` |
| tests | **+13**, from 339 to **352** all-features |
| suites | 194 default / 352 all-features |
| enforcement | the engine-free scan now also asserts the ensemble is inside it |
| carved off | the scale mapping (needs signals), `#39`, `V-01`, where the weights live |
