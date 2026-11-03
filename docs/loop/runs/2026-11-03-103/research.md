# Iteration 103 — research: an escape hatch that is not a trapdoor

## The item

Not an open issue: **a defect in code iteration 100 shipped**, found by asking what the code *does* rather than what its
doc *claims*. `#20`'s acceptance criterion 2 is *"the override, when used, is reported in the query result rather than only
logged"*, and iteration 100 closed `#20` on `Index::overridden`.

## What the doc said, and what the code did

`Index::overridden`'s own doc: *"every search built from this index can see it without a second lookup, and a panel can
show **'loaded with a stamp override'** on every result it draws from here."*

`grep -rn overridden src/semantic` returns the field, its two construction sites, and **no reader**. Both search entry
points began:

```rust
if &index.stamp != loaded { return Err(SearchProblem::StampMismatch); }
```

so an index loaded with `allow_override = true` **could never be queried**. Measured before the fix, with a probe:

```
PROBE after override: Some(StampMismatch)
```

The override loaded an index and then refused every question asked of it. That is worse than not offering it: a user who
follows the refusal's own advice — *"Re-index the library with this model"* — gets an escape hatch whose handle is welded
shut.

## Sources

| Source | What it reports |
|---|---|
| `docs/adr/0004-vector-store.md` (vendored in this repository) | a stamp exists because a vector from the wrong model is **undetectable at query time**; the index must describe the model that made its vectors |
| `src/semantic/index.rs`'s `IndexProblem::ModelMismatch` | carries **both** stamps, so the refusal names what to fix |
| `#20`'s text | *"allow an explicit override for a user who knows what they are doing, and make the override visible in the result"*, and *"make the override visible in the query result rather than only logged"* |
| Vectors from two models compare without complaint | the reason a recorded override must be **narrow** rather than a standing permission |

## The technique

One predicate, in the place that knows the answer — `Index::accepts(&ModelStamp)`:

```text
accepts(loaded)  =  (stamp == loaded)
                 ∨  (overridden is Some(m) ∧ m.loaded == loaded)
```

Two clauses, each earning its place:

1. **The ordinary case** is a matching stamp, unchanged.
2. **The override clause is pinned to the model it was granted for.** An index overridden for `version b` is not
   queryable with `version c`. The user consented to **one specific mismatch**; a third model is a different claim about
   the vectors, and honouring it would turn a recorded override into a standing permission to rank noise — which is
   exactly the bug `docs/adr/0004` exists to prevent.

**Why the rule lives on `Index` and not in the searches.** Two entry points check the stamp (`search` and
`search_ranked`), and a rule written twice is a rule that eventually differs. `Index` already holds both facts the
decision needs — its own stamp and the mismatch it was loaded through — so it is the only object that can answer without
a caller passing state around.

## Parameters

None. This is a predicate over existing data; there is no rate, curve or threshold to justify, and adding one would be a
number the crate does not need.

## What good sounds like

A user who overrides gets **a working index and a visible warning**, not a refusal. A user who does not override gets the
refusal they always got, naming both versions. A user who overrides for one model and then loads a third is refused,
because their consent did not extend that far.
