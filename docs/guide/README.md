# bevy_siren — the guide

Every page here is about one thing a game does with sound, the way `bevy_siren` makes it one line.

- [Cues](cues.md) — name a sound once; the crate chooses the recording, the pitch and the gain.
- [Buses](buses.md) — mix in categories, duck one bus under another, and spend voices on what a player is
  listening to.
- [Music](music.md) — layers that change on the beat or the bar, and stingers that do not wait.
- [Spatial](spatial.md) — place a sound in the world and let distance decide its gain.
- [The Cauldron](cauldron.md) — what `bevy_grimoire`'s audio panel reads and writes in
  `<project>/family/bevy_siren.ron`.

The design rules and their sources are in [the research note](../research/siren.md).
