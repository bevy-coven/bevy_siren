# research.md — iteration 12: a stale voice handle must not take the game down

## The item

`stop_requests` takes entities straight from a **message** and applies a command to each:

```rust
for (entity, why) in requests {
    commands.entity(entity).insert(Stopping { since_s: now, why: … });   // ← panics for a despawned entity
```

Every other `commands.entity(..)` in the crate takes an entity from a live query (`voices.get`, `voices.iter`), which
cannot yield a dead one. This is the one path where the entity comes from **outside**, and `Played::entity` is
documented as the handle to keep:

> `/// The voice entity, for [`StopVoice`] or for a game that hangs its own data on it.`

The crate's own `retire_voices` despawns that entity the frame a one-shot ends. So the ordinary sequence — hold the
handle from `Played`, the sound finishes, the game stops it — panics in a `0.20` `App`:

```
Encountered an error in command: Entity despawned: The entity with ID 43v0 is invalid;
its index now has generation 1.
```

That is a **hard panic in a shipping game**, from an input the crate's documentation invites. It also breaks the crate's
own rule, stated in this repository's notes: *"No `unwrap`/`expect`/`panic!` outside tests."* A panic reachable from a
normal message is the largest violation of that rule the crate has.

## Sources

1. **Bevy 0.20's own contract** (`bevy_ecs-0.20.1/src/error/handler.rs`, the message above). The default error handler
   **panics**; the framework's own advice is `EntityCommands::queue_handled` / `queue_silenced` for an entity that may be
   gone. Bevy's `Entity` is a **generation-checked** id precisely so that touching a despawned one can be *detected* —
   and it gives a caller two ways to handle it. What it does not give is "insert on an unknown entity and carry on".

2. **The crate's rule R6** (`docs/research/siren.md`): *"a play is never silent: `Played` or `Refused`, both with the
   reason."* The subject is a decision about a **sound**. A `StopVoice` for a sound that has already ended is not an
   error in the game's logic — it is the normal race between "the player pressed the button" and "the sound finished".
   The crate already has the right answer for it: the voice is gone, so there is nothing to stop, and nothing to log.

3. **`retire_voices`' own doc** (`src/mix.rs`): a `Mode::Once` cue "is a voice only while it sounds". The moment it stops
   sounding the entity is despawned — by the crate, on the crate's own schedule. Any handle held by a game is stale
   **because the crate made it stale**, which is the strongest possible argument that the crate must tolerate it.

4. **`StopCue` is already safe**, and shows the intended shape: it maps over `voices` (a live query) and can only produce
   live entities. `StopVoice` is the same feature reached through an id instead of a name; the id path forgot the guard
   the name path gets for free.

5. **The same generation concept iteration 7 added**: `model::pack_voice` put a generation in the mixer's voice index so a
   stale index "addresses nothing". Bevy's `Entity` does that natively and panics when the caller does not check. The
   crate checked its own id and not Bevy's — an inconsistency inside one crate about one idea.

**Literature server unavailable, tenth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is Bevy's own error contract and the crate's R6, both vendored here.

## The technique

**A stop request is a decision about a voice that exists now.** Resolve it against the live query before commanding:

```rust
for (entity, why) in requests {
    // An entity a *message* names may already be gone: the crate retires a one-shot the frame it ends, and a game
    // holding `Played::entity` has no way to know. Stopping something that has stopped is not an error to report, and
    // it is certainly not a panic — there is nothing to fade and nothing to log.
    let Ok((_, voice)) = voices.get(entity) else { continue };
    commands.entity(entity).insert(Stopping { since_s: now, why: why.to_owned() });
    log.push(Entry::Stopped { at_s: now, cue: voice.cue.clone(), why: why.to_owned() });
}
```

`voices` is already queried `Without<Stopping>`, so a **second** stop for a voice already fading is also skipped — which
is what the crate already wanted (a fade is begun once). `StopCue` is unchanged: it iterates the same query and its
entities are live by construction.

That is the whole change: one `let … else { continue }` in place of a command that assumed an entity exists, plus a
comment saying why a stale handle is normal rather than a mistake.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what a stale `StopVoice` means | the sound has already ended; nothing to stop, nothing to log | R6; `retire_voices` |
| what Bevy does by default | panics, and recommends `queue_handled`/`queue_silenced` | `bevy_ecs` error handler |
| the guard | the same `Query<…, Without<Stopping>>` `StopCue` already uses | `stop_requests` |
| a second stop for a fading voice | skipped (the query excludes `Stopping`) | existing `Without<Stopping>` |

No new field, no new number, no public type change.

## What good sounds like

Nothing audible changes for a correct game. What changes for an ordinary one is that it **keeps running**:

| sequence | before | after |
|---|---|---|
| play a one-shot, let it end, `StopVoice(handle)` | **panic: entity despawned** | no-op; nothing logged |
| play a loop, `StopVoice(handle)` | fades | unchanged |
| play a loop, `StopVoice` twice | second is skipped | unchanged |
| `StopCue("wind")` with the voice gone | no-op | unchanged |

A reviewer judges it without ears: the probe's four lines, and a test that despawns a voice and then sends `StopVoice`
for its handle — which today panics and after the fix returns.

**The trap this closes is worth naming.** Every other command in the crate takes a queried entity; this one took a
message's. A reviewer looking for "can an id from outside reach a command?" has one place to look, and it is the one
that was wrong.
