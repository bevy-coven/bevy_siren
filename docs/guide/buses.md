# Buses: mixing in categories

A game does not set the volume of a sound; it sets the volume of a *kind* of sound. `SirenSettings` is that table:

```rust
use bevy_siren::*;

let settings = SirenSettings {
    master_db: 0.0,
    max_voices: 32,
    buses: vec![
        BusLevel::new("Music").gain_db(-2.0),
        BusLevel::new("Effects").max_voices(16),
        BusLevel::new("Ambience").gain_db(-6.0),
        BusLevel::new("Dialogue").max_voices(2),
    ],
    // A line of dialogue steps the music back 8 dB and the effects 4 dB while it plays.
    ducks: vec![
        Duck::new("Dialogue", "Music").by_db(-8.0).hold_s(0.25).fade_s(0.6),
        Duck::new("Dialogue", "Effects").by_db(-4.0).hold_s(0.25).fade_s(0.4),
    ],
    ..SirenSettings::default()
};
```

`Music`, `Effects`, `Ambience` and `Dialogue` are the four every project starts with. A cue that names `""` plays on
`Effects`; a cue that names a bus the table does not have is refused, never quietly moved.

## Ducking

A duck is a rule: a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it recover over
`fade_s`. The envelope is read every frame from real time, so a fade keeps running while the game is paused behind a
conversation — and two ducks do not stack: the deeper one wins, because a bus already at −8 dB is not lowered another
4 dB by a second voice asking for −4.

The default pair is the one every game wants, per Gregory *Game Engine Architecture* §14.6.3: dialogue steps the music
back 8 dB and the effects 4 dB, holding a quarter second and recovering over a fraction of a second.

## This frame's mix, readable

`SirenMix` is what the panel draws and what your own meter should read:

```rust
fn meter(mix: Res<SirenMix>) {
    let music = mix.bus_db("Music");          // dB, master and ducks applied; SILENT_DB for a bus nothing started
    let live = mix.voices_on("Effects");      // voices a player can still hear
    let ducked = mix.deepest_duck();          // Some(("Music", -8.0)) while a line plays
    let _ = (music, live, ducked);
}
```

It is one number per bus, not a private float per sink: a bus's level is a decision, and a decision that cannot be
read is a decision nobody can debug.

## The voice budget

`max_voices` is the whole table's budget; a bus's own `max_voices` caps it further (`0` is the table's). A request
stops voices to make room in this order: the cue's own polyphony, its bus's cap, then the table's. Each cap is hard:

- The lowest priority goes first, then the oldest of that priority, so the newest sound is what a player keeps
  hearing.
- A voice at `UNSTEALABLE_PRIORITY` (180: music, stingers, dialogue) is never taken.
- A request that cannot be made room in is `Refused { VoiceLimit { bus } }` and **nothing plays**, so a game that
  says 32 voices gets 32 rather than 33 with a warning.
- **The bus is named only when the bus refused.** `bus: Some(name)` means that bus's own `max_voices` was full;
  `bus: None` means the table's budget or the cue's own `max_instances` was, and every bus may have room. The two
  sentences are ``no voice free on bus `Effects` `` and `no voice free`.

A stolen voice fades over `DEFAULT_FADE_S` (120 ms) rather than stopping dead — a cut is a click — but it stops
counting against the budget the moment the steal is decided, so the new sound starts in the same frame.

**A voice that simply ends frees its slot the frame after its sample does**, on either backend, and the log says
`ended`. That is what makes the budget a budget rather than a lifetime quota: a game that has played 40 footsteps
against a cap of 32 is refused nothing, because each ended footstep gave its voice back (Gregory §14.5.8.4: a voice
that stops *frees its resources for use by other voices*). A loop is the exception on the other side — it has no end
to notice, so it counts until it is stopped.

## Mute

A muted bus is silence, and its level is kept: unmuting returns it. That is why `BusLevel::mute` is a flag and not a
gain of `SILENT_DB` — a player's mute switch must not destroy the mix they spent an hour setting.

## Checking a table

`SirenSettings::problems()` names a bus listed twice or unnamed, a budget of zero, a duck naming a bus the table does
not have, a duck that raises rather than lowers, and a hold or fade that is negative or not a number.

## Two backends, one behaviour

Without the `mixer` feature, Bevy's `AudioPlayer` plays each voice and the mix writes every voice's volume into its
`AudioSink`. The `mixer` feature keeps **one slot per voice heard at once**, reusing the slot of a sound that has
ended — a long session does not accumulate one player per sound played — and each slot hand-out is a new name, so an
entity still fading out cannot address the voice that replaced it. With it, the crate sums one graph and a bus's gain is one number per bus per frame. Either way a play
answers `Played`, a refusal is `Refused`, and a live sound is a `Voice` — so a game, an editor panel and a test all
see the same thing and only the sample path differs.

Two consequences are worth knowing, because both are observable from a game:

- **A voice's gain is written by the mix on both paths.** On Bevy's path its `AudioSink` receives the whole chain —
  cue, compensation, bus and master. On the mixer's, the bus's number goes to the bus and the voice's own gain goes
  to the voice, so a layer that moves on the bar or a voice that fades out is heard at the number `SirenMix` and
  `MusicState` report.
- **A sound's end is noticed on both paths.** Bevy's sink reports an empty sample; the mixer's player reports the
  same. The voice is retired, its `ended` line is logged, and its slot in the budget is free for the next play —
  which is what keeps a full bus a *steal* rather than a lockout.
