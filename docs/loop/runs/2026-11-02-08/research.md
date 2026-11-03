# research.md — iteration 8: a position on a non-spatial cue is a refusal, not a panner

## The item

`answer` applies a cue's attenuation only when **both** `cue.spatial` and `request.position` are set:

```rust
if cue.spatial && let Some(at) = request.position { … }
```

Everything after that ignores `cue.spatial`. `spawn_voice` sets Bevy's `PlaybackSettings.spatial` from
**`request.position.is_some()` alone** (`src/play.rs:627`), so a cue authored `spatial: false` and played with
`.at(position)` goes through Bevy's two-ear `SpatialPlayer`, which multiplies each ear by `min(1, 1/d²)` — a falloff the
crate never computed, never divided out (`Compensation` is 0), and never reported. Probed:

| request | `Played.gain_db` | what is heard |
|---|---|---|
| `Cue::spatial = false`, `.at((0,0,8))` | **0.00 dB** | the panner's own gain: **−38.7 dB** at 8 m, panned |
| `Cue::spatial = true`, no `.at(..)` | 0.00 dB | flat at the cue's gain — **documented behaviour** |

The second row is not a bug: `Cue::spatial`'s doc says a spatial cue "played without a position is heard by everyone at
the cue's own gain". The first row has no such contract, and it is the one that lies: the log says a sound played at
0 dB, the meter believes it, and the sample is 38 dB down.

The crate's own guide states the principle it breaks (`docs/guide/spatial.md`): *"A spatial play with no listener is
refused with the position it was asked for, because playing it flat, or at the origin, is a different sound from the one
the game asked for."* The same argument runs the other way: a cue whose author said it is **not** spatial, played at a
position, is a play asking for something the cue does not do. Today it is quietly done to it anyway, through a law the
project did not write.

## Sources

1. **The crate's stated rule R7-in-waiting** — `docs/guide/spatial.md`'s *"playing it flat, or at the origin, is a
   different sound from the one the game asked for"* — applied symmetrically. A crate that refuses `SpatiallyMute` for
   "asked for a place, no ears" must not silently apply a *different* place law for "asked for a place, cue is not
   placed".

2. **`Cue::spatial`'s own doc** (`src/cue.rs:53–55`): *"Whether the cue is placed in the world. A spatial cue played
   with `Play::at` attenuates with distance; one played without a position is heard by everyone at the cue's own
   gain."* It defines what a spatial cue does with and without a position. It says nothing about a non-spatial cue
   given one — and the code gives it a panner, which is the one thing neither sentence describes.

3. **`rodio` 0.22.2, vendored** (`source/spatial.rs`, mirrored by `model::backend_spatial_gain`): `Spatial::set_positions`
   multiplies each ear by `min(1, 1/d²)` **and** pans. That is a distance law as well as a pan — the crate documents
   this and divides it back out for spatial cues, precisely so a cue's `Attenuation` is the law a project hears. On the
   non-spatial path that division is skipped, so a project hears `min(1, 1/d²)` instead.

4. **Bevy 0.20's `PlaybackSettings::spatial`** (`bevy_audio-0.20.1/src/audio.rs`): a plain `bool` that routes the voice
   through the two-ear player. It is the *only* switch the backend has, which is why the crate's own cue flag must
   decide it, not the presence of a position.

5. **The crate's rule R6** (`docs/research/siren.md`): a play is never silent — `Played` or `Refused`, both with the
   reason. A `Played` whose `gain_db` is 0.00 while the sample plays at −38.7 dB is worse than silent: the meter is
   wrong, so nothing downstream can notice.

**Literature server unavailable, sixth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is argued from the crate's own guide, `rodio`'s vendored source and Bevy's
`PlaybackSettings`, all of which this repository already holds.

## The technique

**One rule: the backend pans a voice if and only if the cue is spatial and the play has a position** — the same
predicate `answer` already uses to apply attenuation. Three places must agree, and today two of them disagree with the
third:

1. `answer` computes attenuation under `cue.spatial && request.position.is_some()` (already correct), and sets
   `Compensation` there.
2. `spawn_voice` sets Bevy's `PlaybackSettings.spatial` from **`request.position.is_some()` alone** — wrong.
3. `spawn_voice` attaches a `Transform` from `if let Some(at) = request.position` — the same wrong predicate, in a
   second place.

The fix makes 2 and 3 use 1's predicate. Then a non-spatial cue with a position is **flat, unpanned, and at the gain
`Played` reports** — which is what `spatial: false` means — and the log gets one line saying the position was ignored,
so a project that passed it by mistake can see it.

Refusing instead of ignoring is a real option and is *not* taken here: it needs a new `Refusal` variant, which is a
public enum change for every consumer's `match`. `needs-decision/non-spatial-position.md` carries it, with the sentence
it would print and what each choice costs.

| request | today | after |
|---|---|---|
| spatial cue, position, listener | panned, the cue's attenuation, `Compensation` applied | unchanged |
| spatial cue, position, no listener | `Refused::SpatiallyMute` | unchanged |
| spatial cue, no position | flat at the cue's gain | unchanged (documented: "heard by everyone at the cue's own gain") |
| **non-spatial cue, position** | **panned at `min(1, 1/d²)`, `Played.gain_db` says 0.00** | **flat, unpanned, and `Played.gain_db` is true; the log says the position was ignored** |

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| when to pan | `cue.spatial && request.position.is_some()` | `answer`'s existing predicate; `Cue::spatial`'s doc |
| the panner's falloff | `min(1, 1/d²)` per ear | `rodio` `spatial.rs`; `model::backend_spatial_gain` |
| that falloff at 8 m, on-axis | −38.7 dB average | computed from the vendored formula, in this run's probe |
| a spatial cue with no position | flat at the cue's gain | `Cue::spatial`'s doc, unchanged |
| a non-spatial cue with a position | flat, no pan, and the log says the position was ignored | **design default**: the crate applies no law a project did not write; the alternative is `needs-decision/non-spatial-position.md` |

## What good sounds like

Nothing should sound different for any cue that did what its author said. A project that wrote `spatial: false` and
passed a position by accident hears the cue at the gain the log reports — flat, no pan — instead of a sound 38 dB down
that its meter called 0. A project that wanted distance writes `spatial: true`, which is the field's whole purpose.

A reviewer judges it without ears: `Played.gain_db` and the audible gain agree in both rows of the probe, and
`render.txt` gains a line naming the ignored position. The measurement that proves the old bug is the 38.7 dB gap; the
measurement that proves the fix is that gap closing to zero.
