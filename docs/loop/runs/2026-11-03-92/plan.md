# Iteration 92 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **`#25` key** | The last feature `#27`'s filter set needs, and ADR 0003's "out of scope" rests on an **unmeasured** 40 % figure. A measurement either confirms the exclusion or overturns it — both are worth more than the next feature. |
| 2 | `#28` the MCP tool host | Largest user-facing win. |
| 3 | Finish `#36` on the GPU | Needs the current commit and the graph on `big`. |
| 4 | Share the spectral/tempo FFTs with `mel` | Two extra passes over the same frames. |
| 5 | Wire tempo and key into `build` and the index | One schema change, after both exist. |

**Why (1) beats (2) in one sentence:** #25 decides whether an ADR's exclusion was right, and a wrong exclusion costs
every consumer a feature they could have had.

## The change

`semantic::key` — `estimate`, `estimate_with`, `Key`, `Relation`, `Compatibility`, `KeyProfile`.

| Decision | Choice | Why |
|---|---|---|
| method | **Krumhansl-Schmuckler**, vendored profiles | the standard, and the twelve numbers are visible rather than cited by name |
| chroma window | **8192 points**, its own transform | 1024 cannot see a pitch; 4096 cannot separate B4/C5 |
| band edges | **`[n − 0.5, n + 0.5)`** | a note is centred on its MIDI number; the first version was half a semitone out |
| absence | **`Key::absent()`** | `#25`'s trap: an impact has no key |
| confidence | **0.15**, measured | between a progression's 0.241 and an impact's 0.067 |
| compatibility | a **named serde type** with cited defaults | `#25`'s third criterion |

## Modules touched

| File | Change |
|---|---|
| `src/semantic/key.rs` | **new** — the detector, the compatibility table, 12 tests |
| `src/semantic/mod.rs` | one `pub mod` |

No new dependency, no index section yet (deliberately, with tempo).

## Tests that prove it

1. `the_corpus_accuracy_is_recorded` — **24 keys**, exact and fifth-compatible both printed (criterion 1).
2. `an_atonal_asset_has_no_key`, `a_single_impact_has_no_key`, `silence_has_no_key` — criterion 2 from three directions.
3. `the_compatibility_table_penalises_a_clash_and_never_an_unknown` — criterion 3.
4. Eight more: relations, naming, refusals, determinism.

## Carved off

- **Transposition** — `#25` puts it out of scope.
- **Key for effects** — also out of scope, and the `Option` enforces it.
- **A real-music corpus.** The 100 % figure is on synthetic tonal material; saying so is more useful than inflating it,
  and the decision file records what a labelled set would add.
