# research.md — iteration 71: the ensemble's arithmetic, and its breakdown

## S1 — what was decididable, and what was not

Iteration 70's remaining list has five items. Three cannot be built:

| Item | Blocked on |
|---|---|
| where the weights live | a definitions-file schema change → `needs-decision/` |
| `V-01` | a person and a licence-cleared corpus |
| `#40`'s full ensemble | `#39`'s encoder → `S-05`'s licence audit |

What remains is `#40`'s **structure**: the types a score is carried in and the arithmetic that makes a breakdown add up.
That needs no signal at all, so it can be built, tested and explained before any encoder exists — and building it first
means `#40` has somewhere to put its numbers instead of inventing them.

## S2 — where the arithmetic belongs, and the rule that enforces it

`#40`'s note says to put the scoring *"where `src/model.rs` keeps its arithmetic"*. That is not a preference:
`tests/all.rs::model_names_no_engine_type` scans **exactly one file** for `bevy::`, `Vec3`, `Entity`, `Res<`, `Query<`.
A score defined in `semantic/` would sit outside that scan and could quietly grow an engine parameter, and then nothing
would test it without an `App`.

So the types went into `src/model.rs`, and a second test —
`the_ensemble_lives_in_the_engine_free_model` — asserts they are **there**, so moving them out is a failing test rather
than a silent exemption. A rule with no test drifts; a rule with one does not.

## S3 — the technique: four facts per signal, not one

Each signal contributes a [`Contribution`] carrying **three** numbers and a flag:

| Field | What it answers |
|---|---|
| `raw` | what the signal saw, in its own units |
| `weight` | how much the project cared |
| `contribution` | `raw * weight` — what entered the sum |
| `missing` | whether the signal had anything to say at all |

**The flag is the subtle one.** A tempo that was never estimated and a tempo that was estimated and disagreed both
contribute `0.0`. Conflating them misleads precisely the person reading the breakdown to understand the ranking — and
that person is why the breakdown exists.

The score is the **sum**, not a mean. The weights are the normalisation: a project that halves a weight is saying that
signal matters half as much, and dividing by the total would undo the statement. `shares()` is the normalised view, for
reporting.

## S4 — the scales problem the issue names

> *"A raw cosine in [−1, 1] and a loudness penalty in LU must be mapped onto a common range before weighting."*

This iteration does **not** solve it, and says so rather than pretending: the mapping is a property of the signals, and
there are no signals yet. What it does provide is the place the mapping will be visible — a `Contribution`'s `raw` is
reported beside its `weight`, so a caller can see that one signal arrives in `[−1, 1]` and another in decibels. **A
breakdown that showed only the products would hide exactly that.**

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| signals per result | **4** | `Signals`: semantic, acoustic, tempo, key |
| numbers kept per signal | **3 + a flag** | measured by design, S3 |
| score | sum of contributions | asserted: 0.4·1.0 + 0.2·0.5 + 0.3·0.35 + 0.1·0.35 = **0.64** |

No new constants: every number here is a caller's, and the arithmetic is a sum. The one authored decision is *sum rather
than mean*, argued in S3.

## What good sounds like

A breakdown is good when a person can read **why a result is first** without opening the code:

> *"semantic carried it (+0.400 from 0.4 raw at weight 1); acoustic counted against it (-0.100 from -0.2 raw); no value for
> tempo, key"*

That single sentence names the signal that carried it, the one that counted against it, the raw values and the weights,
and the signals that had nothing to say — which is the whole of `#44`'s `explain_match` in a string.
