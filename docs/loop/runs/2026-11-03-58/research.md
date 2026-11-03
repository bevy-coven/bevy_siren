# research.md — iteration 58: a layer change that names nothing says so

## The item

`move_layers` applied a `MusicLayers` target with:

```rust
if let Some(layer) = state.layers.iter_mut().find(|l| l.name == name) { layer.target_db = …; }
```

`find` returns `None` for a name the score does not have, `if let` skips it, and **nothing is said** — no log line, no
refusal, no counter. So a typo in a layer name was a change that silently did nothing.

**Measured** (`MusicLayers::new(vec![("nope", -6.0)])`, one frame, a two-layer score): the log held only the layers'
own `played` lines — **`["0.00s played `track.calm` (calm) on Music at -80.00 dB"]`** — and nothing mentioning `nope`.

## Why this is rank 1

It is not a missing report for a legal state; it is a **message the crate accepted and discarded**. The crate's premise is
that a request which cannot be honoured says why: `Play` answers `Played` or `Refused`, a cue problem is a sentence, a
stinger that cannot play is logged. `MusicLayers` was the one message with **no failure path at all** — its type has no
field that could report and its system has no log.

`MusicLayers` is also the message an **editor** sends: the Cauldron's audio panel exists to move layers, and a designer
who mistypes a layer name there gets nothing back.

## Sources

**S1. `MusicLayers`' own doc** — *"`targets` is `(layer name, dB)`: a layer missing from the list keeps its gain."* That
is about a layer missing from the **message**; the crate never said what a layer missing from the **score** means, and
the answer it implemented was "silence".

**S2. `SirenSystems::Play`'s contract** (`src/lib.rs:50`) — *"nothing plays with no explanation, because a sound that
does not happen with no explanation is the hardest bug in a game to find."* A layer that does not move with no
explanation is the same bug in the music half.

**S3. Iteration 9's precedent** — a stinger on the wrong bus has **no `Refusal` variant** and writes a precise prose line
to `SirenLog` instead, because `UnknownBus` would be false. `needs-decision/stinger-refusal.md` records it. This
iteration takes the same route for the same reason: the score is not the mix table, so `UnknownBus` would lie, and a new
enum variant is the break three other open decisions already wait on.

**S4. `find` returning `None` is not an error in Rust, and that is the trap** — `if let Some(..)` reads as
*"apply it if the layer exists"*, which is true and hides that the caller's whole intent was discarded. The guard has to
be an `else`, not an `if let`.

**S5. `SirenLog::push`'s own eviction rule** — *"a refusal outlives a play"*, because `Refused` is exceptional and
`Played`/`Stopped`/`Ducked` happen at frame rate. The sentence written here is a `Refused`, so a busy scene cannot scroll
it away behind footfalls.

## The technique

Turn the `if let` into a `let … else` that writes
`Entry::Refused { at_s, cue: name, reason: "this score has no layer `{name}`, so the change names nothing" }` and
`continue`s. One lookup, not two: the same `find` decides both branches.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| log lines for a misspelled layer, before | **0** | measured |
| log lines after | **1 per fire**, naming the layer | measured |
| a real layer in the same change | still moves to its target | measured |
| a stinger's wrong-bus case (the precedent) | log line, no `Refusal` | S3 |
| the log's eviction order | a `Refused` outlives a routine entry | S5 |

## What good sounds like

One sentence per fire, naming the layer the author typed and the score that lacks it —
``refused `cmobat`: this score has no layer `cmobat`, so the change names nothing`` — beside the real layer's row moving
to its new dB. Nothing audible changes: the change did nothing before and does nothing now; it now **says** so.

## Blocked

`hs distill search` answered `500 … gpu busy` for the eighth iteration running, so the sources are the crate's own docs
and `needs-decision/stinger-refusal.md`. No DOI bears on a `find` whose `None` was discarded.
