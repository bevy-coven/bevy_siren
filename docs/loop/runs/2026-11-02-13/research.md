# research.md — iteration 13: `max_instances` is a promise made at the play, not kept every frame

## What this iteration found

Iteration 12's log named the hunt: **hold a public handle across the frame the crate changes it, and use it as
documented**. This pass ran that hunt over the remaining carrier — `Play::on(entity)` — and found it safe in both
orders; then over `MusicState::voice`, `SirenChoices`, and the `Samples` handle, all safe; and then over the one thing
the hunt's own phrasing misses: **a `CueTable` that changes while voices are live**.

Probed:

```
two voices of cue "a" live (max_instances: 2)
the table shrinks cue "a" to max_instances: 1
a third play -> refused VoiceLimit
live voices: 2      <- two are sounding, and the cue says one may
```

`Cue::max_instances`'s doc reads:

> *"How many of this cue may sound at once; `0` is no per-cue limit."*

**Two are sounding at once against a table that says one may.** Nothing ended them, nothing was logged, and the third
play was correctly refused. The field's sentence is a *standing* promise; the implementation keeps it *at the moment of
each play*. Both readings are defensible, and only one is written down — which is the gap.

**This is not a bug to fix by force.** Ending a live voice because a file was edited under it would cut a sound a
player is hearing, on a frame the game did not ask for anything — a worse behaviour than the surprise. The cap is a
**budget for new plays**, and the honest repair is to say so where a project reads it.

## Sources

1. **`Cue::max_instances`' own doc** — the sentence quoted above. It is the only place the field's meaning is written,
   and it describes a state, not an admission rule.

2. **The crate's rule R4** (`docs/research/siren.md`): *"Per-cue polyphony, per-bus cap and a table-wide budget are three
   hard caps. … a request that cannot be made room in is `Refused { VoiceLimit }` with nothing spawned."* Every clause
   is about a **request** — "a request that cannot be made room in" — which supports the request-time reading. R4 is
   the rule; the field's doc is the sentence that disagrees with it.

3. **`tests/mix.rs::a_cue_respects_its_own_polyphony`** asserts at play time: two plays, one voice, and the log says the
   first was stolen. It never edits the table under a live voice, so it cannot distinguish the two readings either.

4. **Gregory, *Game Engine Architecture* 3e, §14.5.8.4 "Instance Limiting"**: *"Instance limiting is a means of
   controlling the number of sounds that are permitted to play simultaneously."* The subject is what is **permitted to
   play** — an admission decision — not a standing constraint on sounds already admitted. That is the source's
   reading, and it is the crate's R4.

5. **The alternative, rejected.** Reconciling live voices against a changed table would mean the crate despawns a
   voice the game did not stop, at a time the game did not choose, and logs a stop the game did not request — three
   surprises to replace one. It would also need a rule for *which* voices go (oldest? lowest priority? the crate
   already has `steal`), and a rule for a table edited back and forth within a frame. That is a feature, not a doc fix.

**Literature server unavailable, eleventh iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 VLLM engine holds the card). No
new paper is claimed; the rule is Gregory's sentence and the crate's R4, both already held here.

## The technique

One sentence on `Cue::max_instances`, one on `CueTable::insert` (the path a project takes to change it at runtime), and a
test that pins the **documented** behaviour so a later change cannot silently pick the other reading:

```rust
/// How many of this cue may **start** at once; `0` is no per-cue limit …
///
/// **A play-time budget, not a standing invariant.** A table edited while voices are sounding does not end them — the
/// cap decides whether the *next* play may start, and the voices already playing live out their length. Ending them
/// would cut a sound on a frame the game asked for nothing, which is a worse surprise than the one it fixes.
```

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what the cap governs | a **request** may start, or be refused | Gregory §14.5.8.4; crate R4 |
| what a table edit does to live voices | nothing | design default, argued above |
| the default | 1 | `Cue::one()`, unchanged |
| the refusal | `VoiceLimit`, nothing spawned | R4, unchanged |

No new field, no new number, no behaviour change.

## What good sounds like

Nothing — no sample, gain or duck moves. What changes is that a project reading the field, or swapping a table at
runtime, is told which of the two readings the crate takes, and a test holds it there. The probe's surprising line —
two voices against a one-voice cap — becomes the documented answer instead of a discovery.
