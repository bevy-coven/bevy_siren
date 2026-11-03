# Iteration 90 — research: hard filters, and the invariant that makes them safe

The item is [#27](https://github.com/bevy-coven/bevy_siren/issues/27). It is **blocked** by #24 (tempo) and #25 (key),
both still open — so this iteration builds the two rules the crate can actually judge, and the tempo and key rules
correctly do nothing because their features are unknown.

## Sources

| Source | The rule it reports |
|---|---|
| `#27`'s scope | a loudness rule (exclude or penalise beyond N LU); tempo and key rules; a **noise-floor rule at the penalty level, not exclusion** |
| `#27`'s criteria | each rule has a pair either side of its threshold; **an unknown feature never excludes**; every threshold overridable; 12 LU refused and 2 LU admitted |
| `#27`'s pitfall | *"when everything is filtered out, the result must say why rather than returning nothing"* |
| `src/semantic/weights.rs` | `loudness_tolerance_db` already exists at **6.0 dB**, documented as a design default |
| iteration 77 | the measured `Loudness { integrated: Option<f32>, .. }` — absent for any clip under 400 ms |
| iteration 89 | `Spectral { noise_floor_db }` |

## The technique, in the sources' terms

A filter is a **policy**, not a scoring term, so it must be readable and arguable: each rule has a threshold on a serde
type, a reason to exist, and a false-positive cost. The false-positive cost here is explicit — `#27` says *"a filter that
excludes the right track is worse than a weak ranking"* — and it drives the one invariant:

**An unknown feature never excludes.** A 0.2 s footstep has no integrated loudness at all: that is a property of the
**measurement**, not of the sound. Excluding it for failing a comparison it was never part of would silently remove the
commonest asset in a library — and silently, because the result set would just be shorter.

So each rule returns one of three verdicts, and only one of them is removal:

| Verdict | Meaning |
|---|---|
| `Admitted` | inside the threshold |
| `Unjudged` | a feature is absent on either side — **never a reason to exclude** |
| `Excluded(reason)` | beyond the threshold, carrying the difference and the tolerance |

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `loudness_tolerance_db` | **6.0** | existing field; a game mix moves in steps of a few dB, and the crate's own duck is 8 dB |
| `noise_floor_tolerance_db` | **12.0** | **design default** — twice the loudness tolerance, because this rule penalises rather than excludes |
| exclusion boundary | `difference > tolerance` | at exactly 6.0 LU the pair is admitted: the tolerance is a limit, not a target |
| noise floor | **penalty only**, never exclusion | `#27`'s own scope |

## What good sounds like

A 12 LU pair is refused with the difference reported; a 2 LU pair is admitted; a 6.01 LU pair is refused and a 6.00 pair
admitted, so the boundary is where the field says it is. A short clip passes every rule untouched and is **counted as
unjudged**, so a caller can tell "the filter found nothing to do" from "the filter allowed everything". And a set where
everything was removed produces a sentence naming the rule, not an empty list.
