# Iteration 90 — review

## Pass 1 — Function: which of #27's criteria hold, and which are honestly not reachable?

**Critique.** Four criteria, and one of them is about a feature that does not exist:

| # | Criterion | State |
|---|---|---|
| 1 | each rule has a pair either side of its threshold, and the result changes across it | **met** — 5.9 / 6.0 / 6.01 / 12.0 |
| 2 | an unknown feature never excludes | **met** — and it is the module's whole design |
| 3 | every threshold readable and overridable | **met** — 1.0 refuses 4 LU, 24.0 admits it |
| 4 | 12 LU refused, 2 LU admitted | **met** |

**What is *not* built, and why that is right.** #27 also asks for **tempo** and **key** rules. Those features do not
exist — #24 and #25 are open — so this iteration builds the two rules the crate can judge and leaves the others
**correctly silent**: a rule over an always-`None` field would be a stub wearing a feature's clothes, and the
`Unjudged` verdict is the honest shape until the measurement lands.

**Critique of the invariant.** *"An unknown feature never excludes"* is the whole safety of a filter, and it is easy to
implement as a guard and get subtly wrong — `!a.is_none() && !c.is_none() && diff > tol` excludes correctly but reads as
though absence were a pass. The module instead **names three verdicts** and has only one of them remove anything, so a
later rule cannot accidentally treat `Unjudged` as `Admitted` without writing code that says so.

**Verdict: GREAT**, for the two rules that exist, with tempo and key explicitly deferred and the reason stated.

## Pass 2 — Zoom: what the compiler and the linter caught

**Critique.** Four defects, and three would have shipped silently.

1. **A test inside an inherent `impl`.** `weights.rs` had no test module, so appending a `#[test] fn` to the file's last
   brace placed it inside `impl Weights`. Clippy: *"the `test` attribute may only be used on a free function"*. **A test
   that never runs is worse than no test**, because it reads as coverage — and this is the fourth fixture-shaped mistake
   in six iterations, after 87's constant vector, 88's `seed % 24` and 89's five-window "short" clip.
2. **The exhaustive literal broke.** `tests/semantic_weights.rs` writes all six fields by hand, so the seventh failed to
   compile. That is the compiler confirming the change is observable, and for a struct a project ships in RON it is the
   correct trade — `..Default::default()` on that literal would hide a new threshold from review.
3. **A broken intra-doc link.** `[Report::excluded]` named a field I never created; the report carries counts. Rustdoc
   refuses a broken link under `-D warnings`, which is why the crate's docs cannot drift from its types.
4. **An unvalidated threshold.** `noise_floor_tolerance_db` needed a `problems()` check, or a negative value would make
   every candidate read as noisier than the anchor.

**The boundary is read closely.** `difference > tolerance` admits at exactly 6.0 and refuses at 6.01, and the test
asserts both — a filter whose boundary is off by an inequality is the kind of defect that survives every visual check.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a person argue with it?

**Critique.** Every number is on `Weights`, which is a project's own RON file:

```rust
loudness_tolerance_db: 6.0,      // a few dB is a mix decision; the crate's own duck is 8 dB
noise_floor_tolerance_db: 12.0,  // design default: twice the loudness tolerance, because this one penalises
```

A project that *wants* loudness jumps — a stylised world — sets the tolerance high and the filter stops removing
anything, which is the right way to disagree with a policy. The noise rule cannot exclude at all, so the worst it can do
is lower a candidate's rank.

**The cascade, which #27 calls a pitfall**, is a sentence:

> `0 of 6 admitted, 6 for loudness. Every candidate was removed, most by loudness: nothing in this library sits within the tolerance of the anchor.`

And `unjudged_loudness` is counted separately, so a library of short effects reads as *"all 3 admitted"* with a note
that two were never judged — which is the difference between a filter that found nothing to do and one that allowed
everything.

**Verdict: GREAT.**
