# Iteration 101 — eval

## Baseline before anything

Nothing red: 198 / 6 / 419 semantic / 102 lib / 6 doc, clippy and fmt clean.

## The cases

`cues.yaml`'s 33 cases unchanged and green — `#23` adds an offline analysis and touches no cue path.

| Case | Green/red | Observation |
|---|---|---|
| a reverberated copy reads wetter | **red before** | no reverb module existed |
| the discrimination is a recorded number | **red before** | idem — and it reads **12/12, bands disjoint** |
| a fully dry clip reports a finite ceiling | **green** | new |
| a clip too short has no measure rather than a confident one | **green** | new — a 40 ms clip is *unmeasured*, not *dry* |
| an empty buffer does not panic | **green** | new |
| determinism | **green** | new |
| the default boundary is ISO 3382's 50 ms | **green** | new |

## The measurement

```
REVERB DISCRIMINATION: 12/12 pairs separated (100 %); dry ≥ 60.0 dB, wet ≤ 1.9 dB; bands disjoint
```

Printed by the test every run, so a regression shows as a changed sentence rather than a changed colour.

## The item

`src/semantic/reverb.rs` — `ReverbSettings`, `Reverb`, `estimate`, `discrimination`, and `DISCRIMINATION_MEASURED`.
