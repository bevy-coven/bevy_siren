# Iteration 96 — research: the ensemble's common scale

## The item

`#40`'s arithmetic exists in `src/model.rs` — `Signal`, `Contribution`, `Breakdown`, `score`, `shares`, `strongest`,
`explain` — and `tests/all.rs` already asserts it lives in the engine-free file. **Nothing constructed one.**
`grep -rn "Breakdown\|Contribution::" src/ --include=*.rs | grep -v '^src/model.rs'` returned **nothing**: the types were
built, tested and unreachable. `#40` is the keystone that blocks `#44`, `#45`, `#47` and `#48`.

## Sources

| Source | What it reports |
|---|---|
| ITU-R BS.1770-4 | loudness in LU; a tolerance is a difference on that scale, which is why `filter`'s `difference / tolerance` is a meaningful fraction |
| Jensen, *Signal Processing* — cosine as `cos θ` | `(c + 1) / 2` is affine in `c`, so it is **order-preserving**; any strictly increasing squash (sigmoid, `c^γ`) is not |
| `src/semantic/filter.rs` (this crate) | `loudness_tolerance_db` 6.0, `tempo_tolerance_bpm` 4.0, `noise_floor_tolerance_db` 12.0 — the thresholds the rules already refuse on |
| `runs/2026-11-03-63/research.md` S3 | the semantic band **overlaps** the misses: correct 0.126–0.429, incorrect −0.104–0.150 |
| `src/semantic/weights.rs` | the four weights and their design defaults, each already justified |

## The technique

`#40`'s own pitfall names the problem: *"a raw cosine in [-1, 1] and a loudness penalty in LU must be mapped onto a common
range before weighting, and that mapping is itself an authored decision."* Every signal becomes an **agreement in `[0,1]`**:

| signal | raw | why |
|---|---|---|
| semantic | `(cosine + 1) / 2` | affine, so `top_k`'s order survives; clamping at the ends touches only the extremes |
| acoustic | `1 − difference / tolerance`, clamped | the same fraction the rule already thresholded, so the two cannot disagree |
| tempo | `1 − difference / tolerance`, clamped | the same shape on `tempo_tolerance_bpm` |
| key | `Compatibility::multiplier` | already `[0,1]`, already authored — no second mapping, so a project that sets `clash_penalty: 1.0` stops the penalty in both places at once |

**Why not a sigmoid.** The measured bands overlap. A squash that sharpens the top of the distribution sharpens the
*wrong* things in a band where a correct match scores 0.126 and a miss scores 0.150. `(c+1)/2` changes units only.

**Why a missing signal is `Option`, never `0.0`.** `Breakdown`'s own doc says it: a zero from a signal that had nothing to
say and a zero from one that was consulted and disagreed are different facts. An asset with no tempo is not an asset
whose tempo disagrees.

## Parameters

No new tunable. Every number is an existing field: `Weights::{semantic, acoustic, tempo, key}` and the three tolerances.

## What good looks like

A result that says *"semantic carried it (+0.710); no value for tempo, key"* and whose parts sum to its score. And an
order that **changes** when a signal is weighted, because a signal that cannot reorder anything is decorative.
