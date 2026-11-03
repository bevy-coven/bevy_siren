# Iteration 84 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **#21's index round trip** | The last p0 in the repository, and the only work whose *decision* already exists. Its recommendation is the same optional-section shape iteration 83 proved, so the cost is now a sibling of code that ships. |
| 2 | Join a sidecar's file names to hashes (#14's last half) | Real, but #14 is closed and this is its remainder. |
| 3 | The binary index writer | ADR 0004's shape; the text form is 3× the size and now larger still. |
| 4 | #22 spectral features | Unblocked, larger. |
| 5 | #28 the MCP tool host | Largest user-facing win. |

**Why (1) beats (2) in one sentence:** #21 is p0 and closes an issue, while #14's remainder refines one that is already
closed — and #21's decision file already names the exact format.

## The change

**`Index` gains `loudness: BTreeMap<u64, Loudness>`**, written as a second optional section.

| Decision | Choice | Why |
|---|---|---|
| absence | `-` in the lufs column | ADR 0003: `−70.0` is both a sentinel and a real level, so a sentinel erases a distinction |
| what is always written | true peak, sample peak, both block counts | a 0.2 s clip has no loudness but does have a peak |
| non-finite level | **refuse the file at parse** | substituting `0.0` would look like a measured silence |
| section order | markers recognised **before** section parsing | measured: the first version parsed `loudness 1` as a provenance record |
| truncation | count checked like every other section | the existing rule, applied again |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/index.rs` | the field, the section writer, the parser generalised to a `Section` enum, `lufs_field`/`db_field`/`parse_loudness_line` |
| `tests/semantic_index.rs` | **4 new tests** — round trip with an absent loudness, no-section compatibility, both sections coexisting, and the refusal of an unrepresentable level |

No new dependency, no new threshold, no `MAGIC` bump.

## Tests that prove it

1. `loudness_round_trips_and_a_short_clip_stays_absent` — `None` comes back `None`, the peak comes back intact, and
   `too_short()`/`silent()` still disagree after the round trip.
2. `an_index_with_no_loudness_writes_no_section` — the additive promise, measured on the bytes.
3. `provenance_and_loudness_sections_coexist` — the marker bug this iteration found.
4. `a_non_finite_level_makes_the_record_unrepresentable` — the file is **refused**, not repaired.

## Carved off

- **Measuring loudness during indexing.** `loudness::measure` exists and is tested; wiring it into `indexer::build` is a
  scan-side change and belongs with the same step that joins provenance to hashes.
- **`f16` and the binary writer.** ADR 0004's remaining work, unchanged by this iteration.
