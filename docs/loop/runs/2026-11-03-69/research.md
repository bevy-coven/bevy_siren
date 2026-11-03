# research.md — iteration 69: measuring a ranking, so a change to one can fail

## S1 — the problem this solves

Every threshold the semantic layer uses is a **design default**:

| Constant | Value | Why it is a default |
|---|---|---|
| `Floor::default().min_score` | **0.0** | no labelled set to choose from |
| `MEASURED_MATCH_FLOOR` | 0.126 | one sample of six clips, five descriptions |
| `MEASURED_MISS_CEILING` | 0.150 | same |

Iteration 63 measured that the correct and incorrect bands **overlap**; iteration 68 measured what that costs — at
0.126 the floor drops 2, 2, 1 and 3 of 3 results. `V-01` is the item that produces a labelled set. **This iteration
builds the thing that consumes one**, because a golden set with no metric is a data file, and the metric is decidable
now while the data is not.

## S2 — the two metrics, and the published figures they follow

| Metric | Definition | What it catches |
|---|---|---|
| **recall@k** | of the descriptions whose right answer is known, the share with it inside the first `k` | the failure that matters: *not finding* a sound |
| **agreement** | of the neighbours a person affirmed fit an anchor, the share the ranking puts above it | a ranking that finds everything and orders it wrongly |

**Recall rather than precision** because a wrong result in a list of three costs a glance and a missing one costs a
developer their afternoon. The same asymmetry is why this crate refuses with a reason instead of playing something
wrong.

**Agreement is positive-only.** A pair a person was shown and rejected is **not** evidence that it is wrong — a person
given twenty candidates rejects most of them for reasons the ranking cannot see. Asserting only what was affirmed makes
the metric weaker and honest; the alternative is a score that punishes a good ranking for a judgement nobody made.

## S3 — the technique, in the terms `V-01` will use

`score` takes **rankings a caller supplies**, keyed by description. It owns no model, tokeniser or index, so:

- its own tests run in **0.00 s** against hand-written orders, which is how the arithmetic is proved;
- the regression harness feeds it real runs without the metric knowing what produced them.

`at_floor` answers the question the labelled set exists for: **if the floor were this, how many right answers
survive?** It is the whole of `V-01`'s purpose in one function.

## S4 — measured on the real model's own cosines

The four rows are iteration 68's `evidence/ask-e2e.txt` — real CLAP, four clips, four descriptions:

| floor | recall@1 | note |
|---|---|---|
| **0.000** | **100 %** | the default |
| 0.126 | **100 %** | `MEASURED_MATCH_FLOOR` |
| 0.150 | 75 % | `MEASURED_MISS_CEILING` |
| 0.500 | **0 %** | the number intuition reaches for |

The table is the argument for `Floor::default() == 0.0` in four rows. It is a test
(`a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model`), not a comment.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| recall@1 with no floor | **1.0** | measured on the real model's cosines, S4 |
| recall@1 at `MEASURED_MATCH_FLOOR` | **1.0** | same |
| recall@1 at `MEASURED_MISS_CEILING` | **0.75** | same |
| recall@1 at 0.5 | **0.0** | same |
| correct-match band | 0.126 – 0.429 | measured, iteration 63 |
| incorrect-match band | −0.104 – +0.150 | measured, iteration 63 |

## What good sounds like

For a metric, "good" is that it **fails for the right reason and names the case**. The report's `describe()` reads:

> *"recall@k 50% over 2 descriptions, agreement 100% over 0 anchors. Missed: `white noise hiss` (rank 2)."*

A count alone would tell a person nothing they could act on. The name and the rank are what make it a regression report
rather than a scoreboard.

The literature search was not run this iteration: the subject is the crate's own measurement machinery, and there is no
external standard for "good recall" that a labelled set does not define better than a paper can.
