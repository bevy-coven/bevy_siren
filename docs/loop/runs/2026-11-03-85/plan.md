# Iteration 85 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **Measure loudness during a build** | Iteration 84's first item. `measure` has been tested since 77 and nothing called it, so the feature stored data only a caller could supply — the "built but not wired" shape this loop keeps finding. |
| 2 | Join provenance's file names to content hashes | The other half of the same wiring, and larger: it needs the scan's path→hash map joined to the sidecar's name→record map. |
| 3 | The binary index writer | ADR 0004's remaining work. |
| 4 | `#22` spectral features | Unblocked, larger. |
| 5 | `#28` the MCP tool host | Largest user-facing win. |

**Why (1) beats (2) in one sentence:** both are wiring, but loudness needs **no new join** — the buffer is already in
hand inside `embed_one` — so it is the cheap half of the pair and it converts a stored field into produced data.

## The change

`embed_measured(path, embed) -> (Vector, Option<Loudness>)` shares one decode between the measurement and the embedding.
`embed_one` becomes a thin wrapper so existing callers are untouched.

| Decision | Choice | Why |
|---|---|---|
| where measured | inside `embed_one`'s existing buffer | a second decode would double the cost on a thousand-file library |
| a failed measurement | **not** a failed file | the vector is the index's reason to exist; loudness is an addition |
| a failed measurement, reported | `Built::unmeasured` and `describe()` | dropping it silently makes a run that measured nothing look complete |
| `Built` | gains **one** field | a public struct; additive, and no caller constructs it outside the crate |

No new field on `CueTable` / `SirenSettings` / `MusicScore`; no new threshold; no new dependency.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/indexer.rs` | `embed_measured`, `embed_one` as a wrapper, `Built::unmeasured`, `describe()` |
| `src/semantic/loudness.rs` | **documentation and test numbers corrected** (7 → 27 for a 3 s clip) |
| `tests/semantic_indexer.rs` | **3 new tests** — measured during a build, a short file stored unmeasured, the summary naming it |

## Tests that prove it

1. `a_build_measures_loudness_for_every_file_it_embeds` — a 3 s tone stores ≈ **−9.0 LUFS** with **27** blocks, and
   `unmeasured == 0`.
2. `a_short_file_is_stored_unmeasured_rather_than_at_the_floor` — a 0.2 s click stores `integrated: None` with a finite
   peak, through a real build rather than only through `to_text`.
3. `the_summary_names_unmeasured_files` — `"0 indexed, 3 unmeasured"`, and a clean run says nothing extra.

## Carved off

- **Provenance's path→hash join.** Separate, larger, and it needs the scan's map; named in the log.
- **The binary writer** and `#22`, unchanged by this iteration.
