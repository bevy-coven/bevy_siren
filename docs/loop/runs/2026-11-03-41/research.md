# research.md — iteration 41: the panel shows a placed sound, and a gain it can be checked against

## The item

`cues.yaml` cases 13 and 14 are about **placement**: a spatial play with no `SpatialListener` is refused
(`SpatiallyMute`), and with one, distance decides the gain — full at the near radius, −6.02 dB at twice it. Both were
tested. **Neither had ever been drawn.** `examples/mixer` had no `spatial: true` cue and no `.at(..)` call: grep found
**zero** occurrences of either, while the crate ships three attenuation models, a panner on Bevy's backend, and a
refusal class of its own. So the one harness a developer opens showed a mix with no distance in it.

Found while making that work: **`Entry::line` wrote a played gain to one decimal.** The panel's row (new) shows `-6.02`;
the log showed `-6.0`. `cues.yaml` case 14's number is `-6.0206`, and to one decimal it is indistinguishable from the
−6.0 dB a cue's *own* `gain_db` might author — so the log could not show the law it was reporting.

## Sources

**S1. `docs/research/siren.md`, default attenuation** — *"`Inverse { near_m: 4, rolloff: 1 }` — a point source that is
full at the listener's feet and halves at 8 m"*. The number this iteration draws.

**S2. `docs/guide/spatial.md`** — the three models as formulae: `None` is 1; `Inverse` is
`1` inside `near_m`, then `near / (near + rolloff·(d − near))`; `Linear { max_m }` is `1 − d/max`, exactly `0` at and past
`max_m`. And *"exactly at the listener's feet inside 4 m, half the amplitude at 8 m, a quarter at 16"*.

**S3. `src/play.rs`** — one predicate decides attenuation, panner and `Transform`:
`cue.spatial && request.position.is_some()`. A missing listener is `Refusal::SpatiallyMute(at)`.

**S4. `tests/play.rs::a_spatial_play_attenuates_with_distance`** — asserts **0.0 dB at 4 m** and **−6.0206 dB at 8 m**
within `1e-3`. This is `cues.yaml` case 14's assertion, in code.

**S5. Gregory, *Game Engine Architecture* 3e §14.5.8.4** (vendored in `docs/research/siren.md`) — voice limits and
stealing; the reason the panel shows `stopped engine.idle: stolen by bang`.

## The technique

1. A **`spatial: true` cue, `bang`**, on `Effects`, priority 30 — the table's one placed sound.
2. A **`spatial` step** writing `Play::cue("bang").at(Vec3::new(0, 0, PLACED_M))` with `PLACED_M = 8.0`. One constant read
   by both the step and the row, so the picture and the play cannot disagree.
3. The **cue row carries the law**: `inverse at 8 m -6.02 dB`, computed with `model::distance_gain` and
   `model::gain_to_db` — the same functions `answer` uses, not a re-derivation in the example.
4. **`Entry::line` writes a played gain to two decimals**, so the log and the row state one number.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `Inverse { near_m, rolloff }` | full inside 4 m, then `near/(near + rolloff(d−near))` | S2 |
| gain at 4 m / 8 m / 16 m | 1.000 / 0.500 / 0.250 | S2, S4 |
| gain at twice the near radius | **−6.0206 dB** | S4 (asserted within 1e-3) |
| the refusal with no listener | `SpatiallyMute(at)`, carrying the position | S3 |
| a played gain's readable precision | **two decimals** | **design default** — one decimal cannot separate −6.02 from −6.0 |
| the step's distance | 8 m = 2 × `near_m` | S1 (the number that makes the law a halving) |

## What good sounds like

A `bang` row ending `inverse at 8 m -6.02 dB` and a log line ending `at -6.02 dB` — the same number twice, from two
different code paths, at a precision where a distance law and an authored gain are not the same string. A `bang` at 8 m
is **6.00 dB quieter** than the same fixture played flat: measured, `cue-bang-0.wav` peaks at −6.087 dB while
`cue-ui-cancel-0.wav` — the same 320 ms, 440 Hz, decay-12 fixture authored at −6.0 dB — peaks at −12.087 dB. A plot of the
law is a flat line to 4 m and then a straight −6.02 dB per doubling.
