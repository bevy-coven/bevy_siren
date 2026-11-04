# bevy_siren — the guide

Every page here is about one thing a game does with sound, the way `bevy_siren` makes it one line.

- [Cues](cues.md) — name a sound once; the crate chooses the recording, the pitch and the gain.
- [Buses](buses.md) — mix in categories, duck one bus under another, and spend voices on what a player is
  listening to.
- [Music](music.md) — layers that change on the beat or the bar, and stingers that do not wait.
- [Spatial](spatial.md) — place a sound in the world and let distance decide its gain.
- [Semantic selection](semantic.md) — describe a sound and find it, behind the `semantic` feature: a folder becomes a
  content-keyed index, a description becomes a ranked answer, and every result says why.
- [The golden set](golden-set.md) — recall, precision and the similarity floor measured on 2 000 recordings people labelled,
  how to run it, how to grow it, and what it cannot say.
- [The Cauldron](cauldron.md) — what `bevy_grimoire`'s audio panel reads and writes in
  `<project>/family/bevy_siren.ron`.

**Start with the tour**: `cargo run --example demo` plays nine scenes in about half a minute — one per page below — with a
panel showing the crate's own mix, music state and log. `examples/demo_render` writes the same scenes as WAVs plus a
bus-gain plot and a report, which is how the numbers in the root README are measured.

The design rules and their sources are in [the research note](../research/siren.md).
