# Iteration 103 — plan: one rule for whether an index may be queried

## Ranked, across the seven open issues

Every one is blocked outside this repository, so the ranking is of what remains *inside* it:

1. **The override trapdoor** — chosen. It is a **bug in shipped code**: a documented escape hatch that does not work, on
   the path a user follows after a refusal. Bugs rank above features, and this one is reachable by following the crate's
   own advice.
2. `#28` — needs `serde_json` (absent from the graph) and a `leaf.rs` row; a dependency decision, not a build.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` is blocked on a decision only the owner can make, and a server built on an undecided codec
would be discarded. The override bug is fixable here, today, and it makes `#20`'s closed criterion actually true.

## Design

`src/semantic/index.rs`:

- `Index::accepts(&ModelStamp) -> bool` — stamp equality, or a recorded override **granted for that exact model**.

`src/semantic/search.rs`:

- both `search` and `search_ranked` call `index.accepts(loaded)` instead of comparing stamps inline.

No new type, no new field, no API break: `accepts` is additive and the two call sites keep their signatures.

## Fields

None. Nothing is authored, so nothing needs a citation.

## Tests

One new test, `an_overridden_index_can_actually_be_queried`, proving five things:

1. an index loaded through an override **is accepted and ranks its vectors**;
2. without the override the load still refuses (`ModelMismatch`), so the escape is the only way through;
3. a plain index refuses a different model, so nothing was loosened;
4. the override **is pinned**: consenting to `b` does not consent to `c`, and the search answers `StampMismatch`;
5. a matching stamp is accepted with no override recorded.

Plus the existing **31 index** and **8 search** tests, unedited — including
`a_model_mismatch_is_refused_naming_both_versions`, which is the guard this change must not weaken.

## What is carved off

Nothing. The item is a predicate and its two call sites.
