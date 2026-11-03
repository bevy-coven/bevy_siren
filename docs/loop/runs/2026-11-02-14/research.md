# research.md — iteration 14: a score edited at runtime leaves its voices behind

## The item

`MusicScore` is a `Resource` — documented as *"a file a project ships and the Cauldron edits"* — so editing it at runtime
is the ordinary path. `start_layers` reacts to a change by **adding** layers and by dropping removed ones from
`MusicState`:

```rust
state.layers.retain(|l| layer(&l.name).is_some());   // the entry goes
for layer in &score.layers { if absent { state.layers.push(...); plays.write(Play::cue(cue)) } }
```

but it never touches the **voices** those entries own. Two consequences, both silent:

| edit | what the score says | what the player hears |
|---|---|---|
| remove layer `calm` | one layer, `combat` | **both stems still playing**; `calm`'s voice is now ownerless |
| point layer `bed` from `track.a` to `track.b` | the layer is `track.b` | **`track.a`** — `track.b` is never started, and `track.a` never stops |

Probed:

```
after removing calm:  names = ["combat"]      voices still live = 2
after retargeting:    voices = 1 -> ["track.a"]   ever played: ["track.a"]
```

A removed layer leaves a **loop** voice with no owner: `retire_voices` skips loops by design ("a loop has no end to
notice"), so it plays until the process ends. A retargeted layer keeps the old sample and never starts the new one — the
score and the sound disagree, and nothing logs either.

This is rank 1: the audible mix is not the mix the score describes. It also compounds: a game that swaps scores between
scenes accumulates one orphaned stem per removed layer, and each is a voice the budget counts.

## Sources

1. **`MusicScore`'s own doc** (`src/music.rs`): *"With the `serde` feature this is a file a project ships and the
   Cauldron edits, beside the cue table."* A file the editor edits is the definition of a value that changes at runtime.

2. **The crate's rule R5** (`docs/research/siren.md`): *"A score is layers of the same performance … A change is
   quantised to the next beat or bar."* A layer is a **voice** in that score; a score that no longer lists it cannot
   have it sounding.

3. **`start_layers`' own doc**: *"**Starts one looping voice per score layer** the first frame a score exists, by
   writing a [`Play`] per layer."* The word is **per layer**; a layer that no longer exists is not one.

4. **`retire_voices`' rule** (`src/mix.rs`): a `Mode::Loop` voice *"lives until it is stopped, because a loop has no end
   to notice"*. That is right for a loop a score owns, and it is exactly why an ownerless loop must be **stopped** by
   the system that orphaned it — nothing else ever will.

5. **The crate's rule R6**: *"a play is never silent: `Played` or `Refused`, both with the reason."* A voice that stops
   because its layer left the score is a decision, and it earns a `Stopped` line like every other — `"layer removed"` /
   `"layer retargeted"`.

6. **Khan et al. 2023, 10.48550/arXiv.2303.15734** (R5's source, read in iteration 0): adaptive music is *"the stems of a
   single piece … mixed in and out according to the game state"*. The stems are the score's layers; a game state that
   changes the score changes which stems are running, not only their gains.

**Literature server unavailable, twelfth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt (a 14.8 GB VLLM engine holds the card).
No new paper is claimed; the rule is the crate's R5/R6 and Khan's stems, already held here.

## The technique

**A layer's voice belongs to the score's layer, and follows it.** When the score changes, `start_layers` reconciles:

1. **A layer that left the score** has its voice stopped — through the crate's one stop path, `StopVoice`, so it fades
   over `DEFAULT_FADE_S` rather than clicking — and its entry is dropped (as today). The `Stopped` line says
   `"layer removed"`.
2. **A layer whose `cue` changed** is stopped the same way and its entry replaced, so the new cue starts. The line says
   `"layer retargeted"`.
3. **A layer whose `gain_db` changed** keeps its voice: the gain is the thing `move_layers` already moves, and a score
   edited to move a layer is a crossfade, not a restart.

`start_layers` already runs in `SirenSystems::Choose`, before `Play`, so a stop written there is read by `stop_requests`
in `Mix` the same frame; a start written there is answered by `Play` the same frame. The two halves land together —
which is what a score edit means.

The voices are found from `MusicState::voice(layer)`, which already exists and is already public.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| what owns a layer's voice | the score's layer of that name | R5; `start_layers`' doc |
| the fade a removed layer gets | `DEFAULT_FADE_S` = 0.12 s | the crate's one stop path, `StopVoice` |
| a gain edit | the voice stays; `move_layers` moves it | R5, unchanged |
| a cue edit | the voice restarts on the new cue | design default, argued below |
| the log | `Stopped { why: "layer removed" \| "layer retargeted" }` | R6 |

## What good sounds like

The **score and the sound agree** for every edit:

| edit | before | after |
|---|---|---|
| remove a layer | its stem plays forever, ownerless | fades in 0.12 s; one `stopped … layer removed` line |
| retarget a layer's cue | the *old* sample plays, the new one never starts | the old fades, the new starts; two lines |
| change a layer's gain | the voice eases | unchanged |
| add a layer | the new stem starts | unchanged |

A reviewer judges it without ears: the probe's `names` and its `voices` list must name the same cues, whatever the score
has been edited to. Before this change they disagree in two of the four rows.
