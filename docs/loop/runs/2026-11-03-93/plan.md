# Iteration 93 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **Wire tempo and key into `build` and the index** | Both features exist and are tested, and neither is called. `#27`'s remaining two rules cannot read a feature that is never produced. |
| 2 | `#27`'s tempo and key rules | Unblocked by (1) — and this iteration is what unblocks them. |
| 3 | `#28` the MCP tool host | Largest user-facing win. |
| 4 | **One shared spectral pass** | Three features now each walk the frames; the cost is measured and growing. |
| 5 | `V-01`'s labelled set | The only thing that turns synthetic accuracy into real accuracy. |

**Why (1) beats (3) in one sentence:** #28 would expose an index whose tempo and key are always empty, which is a worse
first impression than a missing tool.

## The change

`Index` gains `tempo: BTreeMap<u64, Tempo>` and `key: BTreeMap<u64, Estimate>`, each a **fifth and sixth optional
section**, in both formats; `embed_measured` produces both from the shared buffer.

| Decision | Choice | Why |
|---|---|---|
| an absent value | **written**, as `-` | `bpm: None` and *no record* are different facts; a filter reads the first as unjudged |
| an empty section | **not written** | every earlier index is byte-identical and loads |
| key record | the **`Estimate`**, not the bare `Key` | the confidence is part of what was measured, and `#27` may want it |
| production | inside `embed_measured` | the fourth and fifth consumers of one decode |
| a too-short clip | **no record**, not a record of absence | it was never measured, which is different from measured-and-absent |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/index.rs` | two fields, two sections in text and binary, four parse helpers |
| `src/semantic/indexer.rs` | `Analysed` gains `tempo`/`key`; `build` stores them |
| `tests/semantic_index.rs` | **2 new tests** — round trip in both formats; no section when empty |
| `tests/semantic_indexer.rs` | **2 new tests** — a build produces them; a too-short clip still embeds |

## Tests that prove it

1. `a_build_produces_tempo_and_key` — a click track stores **120 BPM ±2** with no caller supplying it.
2. `a_clip_too_short_to_analyse_still_embeds` — 0.1 s stores a **tempo record with no bpm** and **no key record**, which
   is three distinct states preserved.
3. `tempo_and_key_round_trip_in_both_formats` — including both absences, and an unmeasured asset having no record at all.

## Carved off

- **`#27`'s tempo and key rules.** They are the consumer, and they are the next iteration.
- **One shared spectral pass.** Named, measured, and not this iteration — the cost is reported so the next one can
  justify it.
