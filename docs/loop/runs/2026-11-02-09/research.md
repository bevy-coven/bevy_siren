# research.md — iteration 9: a duck waits for a sound that plays

## The item

`play_stingers` writes the stinger's cue and fires its duck in the same breath:

```rust
plays.write(Play::cue(request.cue.clone()));
clock.manual("Music", request.duck_db, now, request.duck_s, DEFAULT_FADE_S);
log.push(Entry::Ducked { … by: "stinger" … });
```

`Play` is answered by `play_requests` **later in the same frame** (`play_stingers` is in
`SirenSystems::Choose`; `play_requests` is in `Play`). When the play is refused, the duck has already been armed and
`Ducked` already logged. Probed, two causes of the same refusal:

| case | Music | `Played`/`Refused` | log |
|---|---|---|---|
| cue not in the table | **ducked −9.00 dB** | `UnknownCue` | `stinger ducked Music by -9.0 dB` |
| cue on `Music`, its sound key has no handle | **ducked −9.00 dB** | `MissingSound { sound: "stinger_01" }` | `stinger ducked…` **and** `refused …` |

The log is the clearest evidence: it contains, on adjacent lines, *"stinger ducked Music by -9.0 dB"* and
*"refused `music.stinger-no-handle`: no handle …"*. The crate is stating a thing happened and that it did not.

This is the loop's worst class: **a duck that does not correspond to a sound**. The crate's own guide says a stinger
*"ducks the `Music` bus **while it runs**"* — and it does not run. Nothing is playing, and the music steps back for it.

## Sources

1. **Gregory, *Game Engine Architecture* 3e, §14.6.3** (this crate's duck rule, R3): a duck is *"a crossfade between two
   bus levels"* driven by **a voice that is playing**. The crate's `DuckClock::fire_all` implements exactly that for
   dialogue — it fires when a voice **starts**, inside `play_requests`, after the play is answered. The stinger path is
   the one place that arms a duck without a voice.

2. **The crate's rule R3** (`docs/research/siren.md`): *"A voice starting on a **source** bus lowers a **target** bus by
   `by_db` for `hold_s`, then recovers over `fade_s`."* The subject is **a voice starting**. A stinger whose play is
   refused starts no voice, so by the crate's own rule it lowers nothing.

3. **The crate's rule R6**: *"a play is never silent: `Played` or `Refused`, both with the reason."* The refusal exists
   and is correct. What is wrong is that the **other** record — `Ducked` — contradicts it. Two messages about one
   request cannot disagree.

4. **The guide** (`docs/guide/music.md:65–67`): *"A stinger plays once, now … and ducks the `Music` bus while it runs.
   It must be a `Music`-bus cue; one that names another bus is refused with a sentence in the log rather than played
   somewhere unexpected."* The bus check exists and works — but a cue **not in the table** has no bus at all, and
   `unwrap_or("Music")` gave it the one bus that passes.

5. **Bevy 0.20's messages and system sets** (`bevy_ecs`; the crate's own `lib.rs:196–199`): `adopt_layers` already runs
   `.after(SirenSystems::Play).before(SirenSystems::Mix)`, which is exactly the window a stinger's duck belongs in once
   it depends on `Played`. The machinery exists; the stinger path does not use it.

**Literature server unavailable, seventh iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is argued from Gregory §14.6.3's "a voice that is playing" and the crate's own R3 and
R6, all of which this repository already holds.

## The technique

A duck is a fact about a **sound that plays**. So the stinger's duck is armed when the answer says it played, not when
the request is written:

1. `play_stingers` checks the cue **exists** first. A cue not in the table is refused with a sentence naming it — the
   same treatment as a cue on the wrong bus, and it removes the `unwrap_or("Music")` that let an unknown cue pass a bus
   check. No ordering needed, because the table is right there.
2. For a cue that exists, `play_stingers` records the request as **pending** (cue, `duck_db`, `duck_s`) and writes the
   `Play`. A new system — `arm_stinger_ducks`, running after `Play` and before `Mix` — reads `Played` and arms the duck
   for any pending request whose cue was **answered**, then logs `Ducked`. A `Refused` for that cue drops the pending
   request, and the existing `Refused` line is the whole story.
3. The pending list is cleared every frame it is read, so a `Play` refused for any reason leaves nothing behind.

Two of those three are ordering; the first is not, and it is the one that made the unknown cue look like a `Music` cue.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what arms a duck | **a voice that started** | Gregory §14.6.3; crate rule R3 (`fire_all` in `play_requests`) |
| a stinger's hold | `duck_s` | iteration 3, `Stinger::duck_s`'s doc |
| a stinger's release | `DEFAULT_FADE_S` = 0.12 s | iteration 3, design default |
| a cue on the wrong bus | refused, log line | guide, unchanged |
| **a cue not in the table** | **refused, log line** | new: the bus check cannot answer for a cue that has no bus |
| when the duck is armed | after `Play` answers `Played`, before `Mix` | design default, matching `adopt_layers`' window |

## What good sounds like

A stinger that cannot play leaves the music **where it is**: no dip, no recovery ramp, and no `Ducked` line claiming
otherwise. A stinger that plays ducks exactly as before — same depth, same hold, same release, on the same frame it
fires, because `arm_stinger_ducks` runs before `update_mix` reads the clock.

A reviewer judges it without ears: the probe's three lines become the truth — `Music ducked: 0.00 dB`,
`refused UnknownCue`, one log line. The **positive** case is the one that must not change, and it is measured beside it:
a cue on `Music` with a handle still ducks −9.0 dB on the frame it fires.
