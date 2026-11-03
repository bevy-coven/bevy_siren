# Iteration 94 — research: tempo and key rules, and a tolerance derived from an estimator's uncertainty

The item is the last two rules of [#27](https://github.com/bevy-coven/bevy_siren/issues/27), unblocked by iterations 91–93
which built the tempo estimator, the key detector, and the storage that produces both.

## Sources

| Source | The rule or number it reports |
|---|---|
| `#27`'s scope | a **tempo** rule (*"incompatible tempi are excluded when both are known"*) and a **key** rule (*"incompatible keys are penalised by the compatibility table"*) |
| `#27`'s criteria | a pair either side of every threshold; **an unknown feature never excludes**; every threshold overridable; 12 LU refused and 2 LU admitted |
| `#27`'s opening | *"mixing a 90 BPM ambience loop under a 128 BPM combat track is a musical clash regardless of how well the timbres match"* |
| `#24`, iteration 91 | tempo's accuracy floor is **±2 BPM at 80–160** (ADR 0003) |
| `#25`, iteration 92 | `Compatibility { clash_penalty: 0.5, parallel_penalty: 0.85 }`, a named serde type |
| `src/semantic/weights.rs` | `loudness_tolerance_db: 6.0` and `noise_floor_tolerance_db: 12.0`, both design defaults with their reasoning |

## The technique, in the sources' terms

Two rules, and they differ in kind — which is `#27`'s own division:

- **Tempo is a hard exclusion**, because two tempi that far apart *cannot* play together. The threshold is not chosen
  from taste: the estimator carries **±2 BPM** of uncertainty (ADR 0003's floor), so a tolerance of **4 BPM** is twice
  that. Below it, a difference may be entirely measurement error, and excluding on it would reject tracks whose tempi
  merely *look* different — the false-positive cost `#27` says is worse than a weak ranking.
- **Key is a penalty, never an exclusion**, because a modal piece sits anywhere and key is the least reliable feature
  here. The multiplier comes from `Compatibility`, which is already a serde type on `Weights`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `tempo_tolerance_bpm` | **4.0** | **design default** — twice the ±2 BPM estimator floor |
| `compatibility` | `clash_penalty: 0.5`, `parallel_penalty: 0.85` | `#25`'s type, already shipped |
| tempo rule | **exclusion** | `#27`'s scope |
| key rule | **penalty** | `#27`'s scope |
| an unknown on either side | **no verdict**, from both rules | `#27`'s second criterion |

## What good sounds like

A 90 BPM loop against a 128 BPM track is **refused** (38 BPM apart) — the issue's own example. A 3.9 BPM gap is
**admitted** and a 4.1 BPM gap refused, so the boundary is where the field says. C major against C# major is
**penalised by 0.5 and still returned**, because a key clash is a worse neighbour rather than an impossible one.

And both rules are **silent on an absent feature**: a track with no tempo record, or a record whose `bpm` is `None`
because it was judged non-rhythmic, compares with nothing. **`bpm: None` is not a tempo** — the distinction iteration 93
made storable is the one this iteration has to read correctly, and reading it wrong would compare every non-rhythmic
asset against every other as though they shared a tempo.
