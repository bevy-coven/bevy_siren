# Iteration 104 — plan: the filter that silently matched nothing

## Ranked

1. **The two `tools` defects** — chosen. They are **bugs in shipped code**, ranked first for that reason alone, and both
   are reachable by a caller doing exactly what the issue documents. `#43` is closed, so nothing else will catch them.
2. `#28` — needs `serde_json` (absent from the graph) and a `leaf.rs` row: a dependency decision.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` is blocked on a decision only the owner can make. These two defects are fixable here, today, and
one of them makes a documented tally field permanently zero.

## Design

`src/semantic/weights.rs`:

- `Weights::min_similarity: f32`, default `0.0`, documented as `Floor::min_score`'s default moved onto a named field, with
  the measured reason a positive default would be wrong.
- `problems()` refuses a `min_similarity` outside `[-1.0, 1.0]`, since a cosine cannot reach beyond that: a project that
  set `2.0` would silently drop everything, which is the same silent-empty failure this iteration fixes.

`src/semantic/tools.rs`:

- the floor check reads `weights.min_similarity` instead of `Floor::default()`.
- the category comparison lowercases **both** sides.
- `Tally::describe` already reports each count; the fix is that the counts can now be non-zero for the right reasons.

## Fields

| field | type | default | citation |
|---|---|---|---|
| `min_similarity` | `f32` | `0.0` | `Floor::min_score`'s existing **design default**; the measured overlapping bands are why it stays `0.0` |

## Tests

- **`"Footsteps"` matches a pack recorded as `Footsteps`** — the case the old code failed.
- a floor raised above a candidate drops it, **and `below_floor` counts it** — the tally field that could never fire.
- a floor of `0.0` keeps everything, so no existing caller changes behaviour.
- `problems()` refuses a `min_similarity` beyond a cosine's range.
- a category that matches nothing reports `wrong_category` and **not** `excluded`, so the fault is assigned to the right
  rule.

## Carved off

Nothing. Two call sites, one new field.
