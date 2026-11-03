# research.md — iteration 34: a music layer could play on the effects bus

## The item

`MusicLayer::cue`'s doc:

> *"The looping cue this layer plays. **It should name the `Music` bus.**"*

**Advice, and nothing checked it.** A layer naming a loop on `Effects` measured:

```
score: layers [calm -> "wrong.bus" (a loop on Effects), combat -> "track.ok" (Music)]
  problems(): []                                        <- the report said nothing
  voices:     ["wrong.bus on Effects", "track.ok on Music"]
  MusicState::live() = 2                                <- both called music
  mix: Effects 1 voices, Music 1 voices
```

So the **score and the sound disagreed**: `MusicState` said two layers were live, one of them was mixed on the effects bus,
and the crate's own report — the thing an author reads — was silent. A layer is a **stem of one performance**; a stem on
another bus is not part of that performance, and `MusicLayers` moving it would move a level on a bus the score never
mentions.

**The stinger already refuses this**, in `play_stingers`:

```rust
if bus != "Music" {
    // **No `Refusal` variant says this** … a host reading `Refused` cannot tell this from a play it never got
    let reason = format!("a stinger plays on Music, but `{}` names bus `{bus}`", request.cue);
    log.push(Entry::Refused { at_s: now, cue: request.cue.clone(), reason });
    continue;
}
```

The layer path had neither the check nor the line.

## Sources

**S1. `MusicLayer::cue`'s own doc** — *"should name the `Music` bus"*. The sentence states the invariant; `should` is not
a mechanism.

**S2. S2 of `siren.md` (Khan et al. 2023)** — adaptive music is *"a vertical layering technique, where the stems of a
single piece are"* keyed to game state. Stems of **one piece** are mixed together or they are not stems of that piece. A
stem on another bus is a different group with a different level, and a crossfade between layers cannot balance it.

**S3. `play_stingers`' enforcement, and its comment** — the same rule, already refused on the stinger path, with the reason
that **no `Refusal` variant says it** and the log carries the sentence instead. The layer path is the same fault with a
quieter mechanism: it *starts* the sound rather than refusing it.

**S4. `MusicScore::problems` returns `Vec<String>`** — unlike `CueTable`'s `Vec<CueProblem>`, so a new sentence is **not**
a public-enum change. Iteration 19 added the gain sentence the same way; this is the seventh score fault.

## The technique

Two changes, one invariant:

1. **`start_missing` refuses a layer whose cue is not on `Music`**, reusing the machinery iteration 16 built for a
   non-looping layer: one notice line per layer, in `MusicScore::problems`' own words, and the layer is not started.
   The `bad` list becomes `(name, cue, sentence)` so **the reason is carried** rather than assumed — both faults now share
   one loop.
2. **`problems()` gains the seventh sentence**, word for word the line the log prints, so a panel and the report cannot
   disagree.

**Why refuse rather than mix it anyway?** Because the alternative is what the probe measured: `MusicState` reporting a
layer live while the mixer puts it on another bus, and `MusicLayers` silently unable to move it. A stem the crate cannot
mix as music is not a stem it should start.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a layer's cue *should* name the `Music` bus | — | S1 |
| stems of **one piece** are mixed together | — | S2 |
| a stinger on the wrong bus is refused with a sentence, not a `Refusal` | — | S3 |
| the score's report is `Vec<String>` — a sentence is not an API change | — | S4 |

## What good sounds like

A score whose layer names a loop on `Effects` does not start that layer, and both the report and the log say, in one
sentence, which layer, which cue and which bus — while the layers that do name `Music` play exactly as before. No sound
changes for a score that was already right.
