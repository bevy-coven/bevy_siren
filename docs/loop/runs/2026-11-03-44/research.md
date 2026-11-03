# research.md — iteration 44: a layer waiting for a voice says so

## The item

A music layer is a **virtual voice** when the budget has no room for it: `MusicState` records `blocked_at` — the live
voice count at the frame its `Play` was refused — and retries only when the count falls below that number. Until then
the layer sounds nothing.

**The panel cannot see it.** `blocked_at` is private with no accessor, so the `Music` block draws
`calm -80.0 dB` for a blocked layer and `calm -80.0 dB` for a layer that is legitimately silent — the same string for
two different states, one of which is a refusal. That is the silent-refusal class: the one question a panel answers,
*"why is the music quiet"*, has two answers and it prints neither.

## Sources

**S1. Gregory, *Game Engine Architecture* 3rd ed. §14.5.8**, vendored at `docs/research/siren.md` line 16 — *"a
**virtual voice** is a sound the game believes is playing but that has no hardware voice"*. The name the crate's
`blocked_at` field is an implementation of, and the reason a panel must distinguish it: the game believes the layer
plays.

**S2. `MusicState::blocked_at`'s own doc** — *"The live voice count at the frame this layer's start was **refused**, or
`None` when it was never refused. A virtual voice is promoted when a voice frees, not every frame: retrying while the
budget is unchanged would write one refusal line per frame into a log a player's panel reads."* The state exists and is
deliberate; only the **reading** is missing.

**S3. `MusicState::live()`** — *"How many layer voices are live."* The complement: a panel that showed `2 of 2` against a
score of two layers would answer the question. It draws neither number.

**S4. `Refused`/`Refusal::VoiceLimit`** — the crate's own vocabulary for "no room", written once per refusal and then
suppressed while the count is unchanged. A person watching a still has no log line, because the line fired before the
capture's window.

## The technique

`MusicState::blocked(&self, layer: &str) -> Option<usize>` — the live count at which the layer's start was refused, or
`None`. An **additive method** on an existing public type: no variant, no field, no exhaustive `match` breaks.

The panel marks the layer with `waiting` in the same place it draws the gain, so one line says both facts:
`calm -80.0 dB waiting   combat -6.0 dB`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a virtual voice | a sound believed playing with no hardware voice | S1 |
| when it is promoted | the live count falls **below** the count at its refusal | S2 |
| layers with a live voice | `MusicState::live()`, never drawn | S3 |
| the marker's word | `waiting` | **design default** — the layer is not "muted" (a bus is), not "stopped" (the game asked), and not "silent" (a gain is); it waits for room |

## What good sounds like

A `Music` block whose blocked layer reads `calm -80.0 dB waiting`, and whose playing one reads `combat -6.0 dB` — two
states, two strings. A layer that is silent *because its gain is silent* still reads only its gain, so the word is never
noise.
