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

A duck is a rule about a **voice**: a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it
recover over
`fade_s`. The envelope is read every frame from real time, so a fade keeps running while the game is paused behind a
conversation — and two ducks do not stack: the deeper one wins, because a bus already at −8 dB is not lowered another
4 dB by a second voice asking for −4.

**A depth and its cause come from the same rule.** Two ducks on one bus mean the depth — the winner — is often *not* the
newest rule. `SirenMix::ducked` is the minimum over every active rule, while `SirenLog`'s `Ducked` entries are newest
first, so a panel that prints one beside the other misnames the cause: a −8 dB dialogue duck under a newer −6 dB
stinger reads as *−8 dB by the stinger*. `examples/mixer` names the newest entry's `by`, gives **its own** `by_db` when
that differs from the depth (`duck -8.0 dB (newest: stinger at -6.0)`), and uses the plain `by <cue>` when one rule
explains the number — a rule that is **recovering** from its full depth is still that one rule, not a second cause.

The default pair is the one every game wants, per Gregory *Game Engine Architecture* §14.6.3: dialogue steps the music
back 8 dB and the effects 4 dB, holding a quarter second and recovering over a fraction of a second.

**The hold ends when the voice does.** If every voice on the source bus stops — `StopCue`, a steal, anything that takes
the sound a player could still hear — the hold is over and the recovery begins from that frame, rather than running its
remaining quarter second and *then* fading. A voice that is still playing keeps the full hold; a second one restarts it.
A voice that is merely fading out does not count, which is the same rule `SirenMix::voices_on` uses: what a player hears.

`SirenMix` holds **this frame's** buses: one entry per bus in the table's list, and nothing for a bus that has left it.
A table that lists no buses means the crate's four (`DEFAULT_BUSES`) everywhere — for a play's bus check, for the panel,
and for the mix — so a cue the crate accepts always has a row to be mixed on.

`problems()` is one of those places: it asks `SirenSettings::bus_names()`, the same answer a play's `UnknownBus` check
reads, so a table with no buses reports **nothing** about buses. And a fault is named **once**: two duck rules that name
the same missing bus — which is the crate's own default pair, `Dialogue → Music` and `Dialogue → Effects` — produce one
sentence, while two *different* faults about one bus (a duck starting on it, a duck lowering it) stay two.

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

`max_voices` is the whole table's budget; a bus's own `max_voices` caps it further (`0` is the table's). A cue's
`max_instances` is the same kind of number for one cue — and all three are **budgets on plays**, not standing counts:
they decide whether the *next* request may start, and a voice already admitted lives out its length. Editing a table
while sounds are playing is therefore safe — the new numbers govern the next play, and nothing is cut on a frame the
game asked for nothing. A request
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

## A number that is not a number

The crate has **one rule** for a caller value that is not a number: it is the **neutral** value for what it measures,
never the silencing one. A distance that is not a number is `0.0` m (the listener's feet,
`docs/guide/spatial.md`); a gain that is not a number is `0.0` dB (**no gain applied**); a detune that is not a number
plays at `1.0×`. `model::finite_or` is the helper and `model::played_semitones` reports the detune the device is
actually at, so `Played.gain_db` and `Played.pitch` never disagree with the sound.

Why not silence it? Because R6 says *"a play is never silent: `Played` or `Refused`, both with the reason"*. Before this
rule, `gain_db: NaN` in a table reached `chain_db`, which returns `SILENT_DB` for a non-finite part: the cue played
silent while `Played` reported `-80.00` as though that level were authored, and nothing anywhere said why.

`chain_db` keeps its silencing rule for a **bus level the crate computed** — that is the mix's protection, and a `NaN`
bus level must not make a game audible. The rule changed for the **inputs**, not for the model.

`needs-decision/bad-gain-problem.md` records the two stricter answers not taken here (a `CueProblem::BadGain` and a
`Refusal` variant for a bad request), because both public enums are exhaustive and a new variant breaks every
consumer's `match`.

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
