# plan.md — iteration 19: a music layer's gain is a caller value like any other

## The pick, and why it beats the runner-up

**Pick:** iteration 18 gave the crate one rule for a non-finite caller value and applied it to five numbers. A music
layer's `gain_db` enters by a **different door** and has no rule:

```
MusicLayer { gain_db: NaN }  ->  layer_db = Some(NaN), Voice.base_db = NaN, the sink gets -80 dB,
                                 problems() = []
```

A **silent stem** with a meter at `NaN` and no report — rank 1, and the one place the new rule was not applied.

Beats the runner-up (`Refusal::NotALoop`, iteration 16's item 1): that one is a log line for a mistake **already
enforced**; this one is a stem that is silent and a report that is empty, and — unlike iteration 18's `CueProblem` — it
can be fixed **with no API change at all**.

## The design

1. **`start_layers` normalises a layer's starting gain** with the existing `model::finite_or(gain_db, 0.0)`, so the
   layer starts at the level it would have had, and `layer_db` / `target_db` / `Voice.base_db` are the number the sink
   is at.
2. **`move_layers` normalises every `MusicLayers` target** the same way, so a move to a non-finite level arrives at
   `0.0` dB instead of writing `NaN` into a layer that is already moving.
3. **`MusicScore::problems` reports the seventh score fault**: a layer whose `gain_db` is not a number. This is
   **not** an API change — the score's report is `Vec<String>`, unlike `CueTable`'s `Vec<CueProblem>` — which is why this
   item is complete and shippable without a decision file.

**`chain_db` and `approach` are untouched.** `chain_db`'s silencing rule and `approach`'s jump-to-target are both right
for what they see; the number simply never reaches them non-finite now.

## Fields

None. `SirenSettings`, `CueTable` and `MusicScore` gain no field. `0.0` dB is the identity of addition in dB — "no gain"
— which is the same neutral value iteration 18 used, and is cited there.

## Tests

- `a_layer_whose_starting_gain_is_not_a_number_starts_at_zero` — the score case: `layer_db` is `0.0`, not `NaN`;
  `Voice.base_db` agrees; the stem is audible.
- `a_move_to_a_gain_that_is_not_a_number_arrives_at_zero` — the `MusicLayers` case.
- `a_score_reports_a_layer_gain_that_is_not_a_number` — exactly one sentence naming the layer; and a **good** score
  reports nothing (the "must not happen" half).
- `a_silent_layer_is_still_silent` — the **guard**: `SILENT_DB` is a finite number and must not be normalised to `0.0`,
  or every layer a score starts silent would come in at full level. This is the test that makes the neutral value safe.

## The carve

- **Not** a `MusicLayers` log line for a bad move: it is the same class as `needs-decision/bad-gain-problem.md`'s
  option C, and adding a second reporting mechanism for one message is worse than one.
- **Not** a `Refusal` or `CueProblem` variant: neither is needed here.
