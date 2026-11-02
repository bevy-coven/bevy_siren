# Music: layers that change on the bar

Adaptive music is not a second track. It is the same performance with each instrument's level keyed to game state, so
a fight can start without the music stopping: the tempo, the beat and the musicians are unchanged, and one part gets
louder.

```rust
use bevy_siren::*;

let score = MusicScore {
    layers: vec![
        MusicLayer::new("calm", "track.calm").at_db(-3.0),
        MusicLayer::new("combat", "track.combat"),
    ],
    tempo_bpm: 100.0,
    beats_per_bar: 4.0,
};
```

The plugin starts one looping voice per layer, and the layer at `SILENT_DB` stays in the mix. It costs one silent
voice and it keeps every layer phase-aligned — which is the whole point: an entry that starts from the first beat
sounds like a second song, and one that is already playing sounds like the same band getting louder.

## Moving the layers

```rust
// Bring combat in and calm out, on the next bar.
layers.write(MusicLayers::new(vec![
    ("combat".to_owned(), -3.0),
    ("calm".to_owned(), SILENT_DB),
]).next_bar());

// Or at once, for a cutscene that does not care about the beat.
layers.write(MusicLayers::new(vec![("calm".to_owned(), -12.0)]).now());
```

`When::NextBar` quantises to the next boundary from the tempo alone (`model::next_boundary`) — no beat map, no authoring
tool, and a frame that lands exactly on the bar is *on* it rather than past it. With `tempo_bpm: 0.0` there is no grid,
and every transition is immediate: a pad, not a song.

Gains ease at `MIX_TAU_S` (50 ms), so a layer moves without a click. A layer moved to `SILENT_DB` is not stopped; it is
silent and still playing, which is what makes bringing it back in the same performance.

## Stingers

```rust
stingers.write(Stinger::new("music.stinger").duck_db(-9.0).duck_s(1.5));
```

A stinger plays once, **now** — a punctuation mark that waited for the bar would miss the moment — and ducks the
`Music` bus while it runs. It must be a `Music`-bus cue; one that names another bus is refused with a sentence in the
log rather than played somewhere unexpected.

## As data

```ron
(
    layers: [
        (name: "calm", cue: "track.calm", gain_db: -3.0),
        (name: "combat", cue: "track.combat"),
    ],
    tempo_bpm: 100.0,
    beats_per_bar: 4.0,
)
```

`MusicScore::problems(&table)` names a layer with no name, the same layer twice, a layer naming a cue the table does
not have, a layer whose cue is not a loop (a layer that ends is not a layer), a tempo that is negative or not a
number, and a score with no layers.

## Reading it back

`MusicState` is the plugin's own record, for a panel or a test: `layer_db(name)` is where a layer is now,
`target_db(name)` where it is going, `voice(name)` the entity playing it, `moving()` how many gains are still easing,
and `next_change_s()` the moment the next queued change is due.

## The example

```sh
cargo run --example music
```

A scripted calm → combat → calm, printing each transition and checking that every one landed on a bar (2.4 s at
100 bpm).
