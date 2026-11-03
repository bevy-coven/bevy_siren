# Spatial: a sound with a place

A spatial cue is placed in the world and attenuates with distance. There are two halves, and the crate owns one of them:
Bevy's `SpatialListener` is the ears, and the cue's `Attenuation` is the distance law.

```rust
use bevy_siren::*;

// One listener, usually on the camera:
commands.spawn((Camera3d::default(), SpatialListener::new(1.0)));

let table = CueTable::new(vec![Cue {
    sounds: vec![SoundSlot::new("bang")],
    spatial: true,
    attenuation: Attenuation::Inverse { near_m: 4.0, rolloff: 1.0 },
    ..Cue::new("explosion")
}]);

// Where it happened:
play.write(Play::cue("explosion").at(Vec3::new(12.0, 0.0, 3.0)));
```

## The distance laws

| `Attenuation` | Gain at distance `d` |
|---|---|
| `None` | `1.0` — heard everywhere: a radio, a voice in the player's earpiece |
| `Inverse { near_m, rolloff }` | `1.0` inside `near_m`, then `near / (near + rolloff * (d - near))`. The default: 4 m, rolloff 1 |
| `Linear { max_m }` | `1 - d / max`, exactly `0.0` at and past `max_m` |

The default is `Inverse { near_m: 4.0, rolloff: 1.0 }`: exactly at the listener's feet inside 4 m, half the amplitude at
8 m, a quarter at 16. A distance that is not a number, or is negative, is treated as `0.0` — a caller handing in a `NaN`
transform gets a sound at the listener's feet, not a `NaN` volume that silences the whole bus.

`model::distance_gain` is the function, and `tests/model.rs` pins it at the two distances a person can check.

## No ears is a refusal, not a guess

```rust
play.write(Play::cue("bang").at(position));   // with no SpatialListener in the world
// → Refused { reason: SpatiallyMute(position) }
```

A spatial play with no listener is refused with the position it was asked for, because playing it flat, or at the
origin, is a different sound from the one the game asked for. A game that wants a flat fallback plays the cue without
`at(..)` and gets exactly that.

The same rule runs the other way: **a cue that is not spatial is never placed**, whatever the play passed. Its
`Attenuation` is not applied, it is not panned, and its `Played.gain_db` is the gain it actually plays at. A
non-spatial cue with `.at(..)` is flat — the position is ignored — because the crate applies no distance law a project
did not write, and applying the backend's own was a bug that made a "flat" cue 38 dB down while the log said 0 dB.

## The two backends

With Bevy's `AudioPlayer` (the default), a spatial voice goes through Bevy's two-ear `SpatialPlayer`, which pans by the
distance to each ear *and* divides by distance squared. That is a distance law as well as a pan, so the crate divides
the panner's own gain back out (`model::backend_spatial_gain`, the mirror of `rodio`'s formula) and the cue's
`Attenuation` is the law a project hears. The pan is kept; the accidental falloff is not.

With the `mixer` feature, a rodio source cannot be re-positioned once it is inside a `Player`, so a spatial cue's
attenuation is applied as a gain at the moment of the play — correct then, and not tracking a moving emitter
afterwards. **A game that needs a moving emitter's pan wants Bevy's path, which is the default.** The `mixer` feature is
for a game that wants one summing graph for everything, and it says so.

## What the crate does not own

The listener's transform is the game's — usually the camera's, sometimes a scripted ear for a cutscene. The crate reads
the first `SpatialListener` it finds and never creates one.
