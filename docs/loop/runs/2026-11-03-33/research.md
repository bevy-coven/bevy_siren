# research.md — iteration 33: a queued change keeps the tempo it was queued under

## The item

`When::NextBar` is documented as *"On the next bar, quantised with [`next_boundary`]"*, and `docs/guide/music.md`'s title is
*"layers that change on the bar"*. `move_layers` computes the instant **once**, when the request is read, from the tempo
the `MusicScore` has that frame — and `Pending` stores only `at_s`. So a tempo edit while a change waits leaves the change
scheduled against a bar that no longer exists:

```
score at 160 bpm (bar 1.5 s)
  queue NextBar(combat -> -3.0)      next_change_s = Some(1.5)
  score.tempo_bpm = 80.0   (bar 3.0 s)
  next_change_s = Some(1.5)          <- the MIDDLE of a bar at 80
  a fresh NextBar at 80 would be 3.0
```

The crossfade fires **off the beat** — which is the one thing vertical layering is for: *"an entry that starts from the
first beat"* is out of phase with every other layer.

## Sources

**S1. `When::NextBar`'s doc** — *"On the next bar, quantised with `next_boundary`."* The promise is about a **bar**, and a bar
is a function of the tempo.

**S2. `docs/guide/music.md`** — the section title is *"Music: layers that change on the bar"*; the example writes
`.next_bar().fade_s(2.4)` and calls a bar *"a crossfade a game asks for is usually a bar or part of one"*. A change that
lands mid-bar is not that.

**S3. `MusicScore` is a `Resource`** — *"a file a project ships and the Cauldron edits"*. A tempo edit at runtime is the
ordinary path, and iteration 14 fixed the same class for **layers** (`start_layers` reconciles a changed score); the
**pending queue** kept its old tempo.

**S4. `next_boundary`'s doc** — *"The next musical boundary at or after `now_s`"*. The function is right; the caller froze
its answer.

## The technique

`Pending` remembers **which boundary** it was quantised to (`When`) and **the tempo it used**. `move_layers`, on a frame
whose tempo differs, re-quantises each change that has not yet fired:

```rust
if pending.tempo != tempo && pending.at_s > now {
    let at = match pending.when {
        When::Now => pending.at_s,                       // already applied; nothing to move
        When::NextBeat => next_boundary(now, tempo, 1.0),
        When::NextBar => next_boundary(now, tempo, f64::from(beats)),
    };
    pending.at_s = at.max(now);
    pending.tempo = tempo;
}
```

- **Never earlier than this frame:** `at.max(now)` keeps the change from being pulled into the past by a faster tempo.
- **No work at an unchanged tempo:** the `tempo` guard is what stops a per-frame re-quantise from pushing a queued change
  forward for ever — the guard test asserts thirty frames move nothing.
- **`When::Now` is untouched:** it is applied on its own frame, so there is nothing pending to move.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a change waits for **the next bar**, quantised by `next_boundary` | — | S1 |
| a crossfade a game asks for is a bar or part of one | — | S2 |
| `MusicScore` is edited at runtime — the ordinary path | — | S3 |
| `next_boundary` answers from `now_s` and the tempo it is given | — | S4 |
| a bar at 160 bpm is 1.5 s; at 80 bpm, 3.0 s | — | measured |

## What good sounds like

A score that halves its tempo while a change waits: the change moves to the bar the **new** tempo has, so the crossfade
still begins on the beat. A change queued at an unchanged tempo does not move at all. A `When::Now` change applies on its
own frame. No sound changes.
