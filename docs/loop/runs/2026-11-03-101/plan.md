# Iteration 101 — plan: the dry/wet estimate

## Ranked

1. **`#23`** — chosen: the only open issue whose blocker (`#4`) is closed and which needs neither a GPU nor a JSON
   dependency. The other eight each need one of those, the editor, or a labelled set.
2. `#35` golden set — needs labelled real assets, which is a person's work.
3. `#28`/`#31` — the JSON dependency decision.
4. `#32`–`#34`, `#38` — the editor.
5. `#37`, `#39` — a GPU host with the graph.

## Design

`src/semantic/reverb.rs`:

- `ReverbSettings { early_window_s, late_window_s }` — a serde type; 0.05 s from ISO 3382, 0.5 s a design default.
- `Reverb { early_to_late_db: Option<f32>, peak_at_s }` — `None` is *no late window*, not *dry*.
- `Reverb::CEILING_DB = 60` — a zero denominator reports a finite ceiling, never an infinity.
- `estimate(buffer, settings)` — frames from `analysis::spectra`, peak frame as the direct proxy.
- `Pair`, `Discrimination`, `discrimination(pairs)` — the record `#23` asks for.
- `DISCRIMINATION_MEASURED` — the constant that says *synthetic only*.

## Fields

| field | default | citation |
|---|---|---|
| `early_window_s` | 0.05 | ISO 3382 C50 |
| `late_window_s` | 0.5 | **design default** |
| `CEILING_DB` | 60 | **design default** |

## Tests (7)

Direction on a reverberated copy; discrimination over 12 pairs recorded and disjoint; a fully dry clip reports the
ceiling rather than infinity; a clip too short has no measure; an empty buffer does not panic; determinism; the default
boundary is 50 ms.

## What this does not do

**Not in the default weights.** A synthetic corpus is not evidence about real recordings, and `#23` asks for that candour
explicitly.
