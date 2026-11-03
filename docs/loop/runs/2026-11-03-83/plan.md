# Iteration 83 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **Finish #14: store provenance and expose it on a result** | p0, and the only remaining work whose *decision* already exists. Two criteria wait on it; both recommendations are additive/non-breaking, which this repo's log three times says need not wait. |
| 2 | #21's index round trip | Also blocked on a decision file — the same optional-section shape, so finishing #14 first settles the pattern once. |
| 3 | The binary index writer | ADR 0004's shape, unasserted 3× overhead. |
| 4 | #22 spectral features | Unblocked, larger. |
| 5 | #28 the MCP tool host | Largest user-facing win. |

**Why (1) beats (2) in one sentence:** both are p0 and both are blocked on a decision file, but #14's is **two**
criteria of one issue and its recommendation already mirrors ADR 0004 — implementing it settles the shape #21 will reuse.

## The change

**`Index` gains `provenance: BTreeMap<u64, Provenance>`**, written as a trailing optional section.

| Decision | Choice | Why |
|---|---|---|
| where | after the vectors, inside the **checksummed** body | a hand-edited licence is a corrupt file, not a silent change |
| when written | only when non-empty | an index with no provenance is **byte-identical** to the old format |
| absence | `-`, with a literal `-` escaped `\-` | absence must not be expressible as content, or `#21`'s `−70 LUFS` mistake repeats |
| marker | `provenance <count>` | every vector line begins with 16 hex characters, so the marker cannot collide |
| truncation | section count checked like the vector count | the existing rule, applied to the new section |

**`rank::Match` gains `provenance: Provenance` and loses `Copy`.** Measured blast radius: **one** `Match` literal in the
whole crate (`rank::top_k`), and no consumer iterates by value.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/index.rs` | the field, the section writer, the parser, `one_line`/`escape`/`unescape`/`parse_provenance_line` |
| `src/semantic/rank.rs` | `Match.provenance`, `Copy` removed, one construction site |
| `tests/semantic_index.rs` | **4 new tests** — round trip, additive promise, empty-vs-absent after a file, truncation, and criterion 3 |

No new dependency, no new threshold, no `MAGIC` bump.

## Tests that prove it

1. `provenance_round_trips_and_is_absent_from_an_index_with_none` — every field survives, including a pack name with a
   **space**; and an index with no provenance writes **no section at all**.
2. `an_empty_licence_survives_the_file_distinguishably_from_an_absent_one` — `#14`'s first criterion, through the file.
3. `a_provenance_section_promising_more_records_than_it_holds_is_refused` — truncation, re-checksummed so the count is
   the only thing wrong.
4. `a_search_result_carries_its_provenance` — `#14`'s third criterion, and an unrecorded asset still carrying a value.
5. `a_result_cannot_be_built_without_provenance` — the never-forget property.

## Carved off

- **Linking a sidecar's file names to content hashes** — that needs the scan, and `indexer::build` is where the two meet.
  The type and both halves of the store are done; the wiring is a separate step and is named in the log.
- **A `MAGIC` bump.** Not needed: the section is optional and the old format is a strict prefix of the new one.
