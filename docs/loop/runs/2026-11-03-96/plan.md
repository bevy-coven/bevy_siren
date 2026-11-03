# Iteration 96 — plan: the ensemble wired end to end

## Ranked items across the 37 open issues

Ordered by dependency and by what unblocks the most:

1. **`#40` ensemble, wired** — chosen. Its types exist and nothing constructs them; it blocks `#44`, `#45`, `#47`, `#48`.
2. `#42`/`#43`/`#44` tools (`set_anchor`, `find_compatible_sfx`, `explain_match`) — all read `Breakdown`, so they were
   unimplementable before this.
3. `#47` query latency budget, `#36` throughput target — both now measurable against a real end-to-end path.
4. `#13` chunk long tracks, `#20`'s override field — independent, each needs its own decision.
5. `#28`–`#31` tool surface, `#32`–`#34` editor — need `#8`'s ADR first.

**Why 1 beats 2:** the tools cannot be written against a type nobody constructs.

## Design

New `src/semantic/ensemble.rs`:

- `agreements(cosine, candidate, anchor, judgement, weights) -> [Option<f32>; 4]` — every signal's raw agreement, `None`
  where it had nothing to say.
- `breakdown(...) -> Breakdown` — the weighted version, built through `Contribution::scored` / `::absent`.

`src/semantic/search.rs`:

- `Ranked<'a>` — embedding, anchor, features, k, floor, weights. Grouped because a nine-argument function gets its
  arguments swapped and `anchor` and `k` are both integers.
- `Scored` — hash, similarity (the model's own scale, kept separate), score (the ensemble's), breakdown, provenance.
- `RankedAnswer` — matches, `dropped`, `excluded`, and a `describe()` naming both counts.
- `search_ranked` — stamp check, width check, `top_k`, floor, `filter::evaluate` (hard rules exclude before scoring),
  `ensemble::breakdown`, then **re-sort by ensemble score**.

`src/model.rs`:

- **`impl PartialEq for Contribution`**, hand-written. A derived one compared `raw: NaN` against `NaN` and returned
  false, so two identical results from two identical runs were unequal — `#40`'s determinism criterion failed on any
  result with an absent signal. Measured before the fix.

## Fields

None new. Every number is an existing `Weights` field with its existing citation.

## Tests (`tests/semantic_ensemble.rs`, 7)

- contributions sum to the score, and each contribution is `raw * weight`;
- an absent signal is marked missing with a zero contribution, never a zero raw;
- the breakdown is usable with no `App`;
- two identical calls agree **field for field** (would have failed before the `PartialEq` fix);
- removing the acoustic weight **changes the order** on a fixture where it matters, and does not on one where it does not;
- the semantic map is strictly increasing across `[-1, 1]`;
- tempo agreement falls with distance and is `None`, not zero, when unmeasured.
