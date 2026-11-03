# Iteration 86 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **Join the sidecar to the scan** | Iteration 85's first leftover, and the last unwired reader. `Sidecar::read` is tested and called by nothing, so a folder's licences never reach the index. |
| 2 | The binary index writer | ADR 0004's remaining work. |
| 3 | `#22` spectral features | Unblocked and cheapest now that `embed_measured` is the place to compute them. |
| 4 | `#28` the MCP tool host | Largest user-facing win. |
| 5 | `#36` indexing throughput | `build` now measures, and this adds a sidecar read; the cost is unmeasured. |

**Why (1) beats (2) in one sentence:** a feature that is built, tested and called by nothing produces nothing — which is
the same "built but not wired" shape that cost iterations 84 and 85, and #14 is closed while this remainder is not.

## The change

`build` reads the folder's sidecar **before** embedding anything and joins it to the scan's `Asset.paths`.

| Decision | Choice | Why |
|---|---|---|
| when read | before the embed loop | so an unreadable sidecar is reported even for a folder whose files all embed fine |
| match key | `Asset.paths` normalised `\\` → `/` | the sidecar writes the path a person sees |
| several paths per content | matched if **any** is named | a content that appears twice is one asset |
| unmatched record | `Built::unattributed` | a typo otherwise makes a licence look recorded and attach to nothing |
| unreadable sidecar | `Built::sidecar`, build continues | different from *no sidecar*, and the summary says which |
| `Built` | gains **two** fields | public struct; additive, and only this crate constructs it |

No new serde field, no new threshold, no new dependency.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/indexer.rs` | `read_sidecar`, the join, `Built::{unattributed, sidecar}`, `describe()` |
| `tests/semantic_indexer.rs` | **5 new tests**; two `Built` literals updated for the new fields |

## Tests that prove it

1. `a_sidecar_licence_reaches_the_asset_it_names` — one record, on the right hash, and the **unnamed** file absent.
2. `a_record_naming_an_absent_file_is_counted` — a typo is counted and named in the summary.
3. `an_unreadable_sidecar_is_reported_while_the_build_still_works` — audio indexes, provenance empty, failure carried.
4. `no_sidecar_is_normal_and_says_nothing` — a clean run reads `"1 indexed"`.
5. `a_pack_name_with_a_space_survives_a_build` — through the build **and** through `to_text`/`load`.

## Carved off

- **Deciding which licences are acceptable** (`S-05`), which `#14` puts out of scope.
- **The `\`-separator normalisation beyond one level.** Windows-style paths are normalised as a string replace; a
  sidecar that spells an absolute path does not match, and that is recorded as an unattributed record rather than a
  guessed match.
