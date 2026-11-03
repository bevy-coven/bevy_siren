# research.md — iteration 70: ensemble weights as data

## S1 — the item, and why it is decidable now

`#41` says it is **blocked by `#40`**, but its own scope is what `#40` would consume: a serde type holding the weights and
thresholds the ensemble combines signals with. `#40` needs `#39`'s music-specific encoder, which is blocked on `S-05`'s
licence audit — a decision, not work.

So the weights type is the half that can be built and judged today, and building it first means `#40` has somewhere to
put its numbers rather than inventing them at a call site.

## S2 — the defaults, and the argument for each

None can cite a source: what a library's sounds *fit* is a property of the library, not of the literature. Each carries
the argument for its value instead, and `at_floor` (iteration 69) is the function that would replace them with a
measurement once `V-01` exists.

| Field | Default | The argument |
|---|---|---|
| `semantic` | **1.0** | the signal the feature is named for; the unit the others are relative to |
| `acoustic` | **0.5** | the semantic bands **overlap** (0.126–0.429 vs −0.104–0.150, measured), so production differences are what break a near-tie |
| `tempo` | **0.35** | below `acoustic`, because a tempo estimate is **absent for every effect** — a signal full of holes earns less weight |
| `key` | **0.35** | equal to `tempo`, because nothing in this crate measures which matters more; ranking them would be a magic constant wearing a name |
| `loudness_tolerance_db` | **6.0 dB** | a game mix moves in steps of a few dB — the crate's own default duck is 8 dB — so more than 6 dB apart is a deliberate difference in level |
| `drop_fraction` | **0.0** | iteration 69 measured what a filter costs: recall@1 is 100 % at 0.126, **75 %** at 0.150 and **0 %** at 0.5 |

The `acoustic` and `drop_fraction` numbers are the two that rest on **measurements this crate made**, not on taste, and
their doc comments say which.

## S3 — the line between "impossible" and "unusual"

The issue asks for a refusal of "internally inconsistent" configurations. Where that line falls is the design decision:

**Refused** — the configuration cannot mean anything:

| Problem | Why it is not a preference |
|---|---|
| all four ranking weights zero | nothing ranks |
| a negative weight | a signal meant to attract now repels |
| a non-finite weight | compares `false` against everything, so every comparison silently fails |
| `loudness_tolerance_db <= 0` | every pair is penalised, which filters the library to one asset |
| `drop_fraction` outside **0..1** | asks for a share of the results that does not exist |

**Allowed** — unusual but possible, and the report must not punish it:

- `loudness_tolerance_db: 40.0` with `acoustic: 0.0` — a stylised world whose loudness jumps are **intended**, which is
  the issue's own example;
- `drop_fraction: 0.0` with a tight tolerance — a deliberately narrow palette;
- `semantic: 0.0` — rank a library by production only.

`an_unusual_but_possible_configuration_is_allowed_through` is the test for the second list, and it is the more important
of the two: a report that refuses a legitimate choice is a report people learn to ignore.

## S4 — what the type does not hold

**Not the model's weights.** CLAP's numbers come from a frozen checkpoint; a project that changed them would be
comparing against an index built by different weights, and the index's stamp already refuses that. These are the
ensemble's weights, which are a design decision a project is entitled to make.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| semantic weight | 1.0 | **design default**, S2 |
| acoustic weight | 0.5 | **design default**, argued from the measured band overlap |
| tempo, key weights | 0.35 each | **design default**, S2 |
| loudness tolerance | 6.0 dB | **design default**, below the crate's own 8 dB duck |
| drop fraction | 0.0 | **design default**, from iteration 69's recall table |
| measured recall@1 at 0.126 / 0.150 / 0.5 | 100 % / 75 % / 0 % | measured, iteration 69 |

## What good sounds like

For a configuration type, "good" is that **a file which says something impossible is a message, and a file which says
something unusual is obeyed**. The report reads:

> *"`acoustic` is -0.5: a negative weight makes a signal that should attract repel. Remove the minus sign, or set it to 0
> to ignore the signal"*

That names the field, the value, why it is wrong and the two ways to fix it — the same shape as `Refusal` and
`CueProblem`, which is the point of following the crate's existing pattern rather than inventing a fourth.
