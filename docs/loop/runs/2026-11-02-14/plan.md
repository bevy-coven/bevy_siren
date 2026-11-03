# plan.md — iteration 14: a score edited at runtime reconciles its voices

## The pick, and why it beats the runner-up

**Pick:** `MusicScore` is a resource a project and the Cauldron edit, and `start_layers` only ever **adds** layers. Two
edits silently break the score/sound agreement:

- **removing** a layer leaves its looping voice playing forever, ownerless (`retire_voices` skips loops by design);
- **retargeting** a layer's `cue` keeps the **old** sample — the new one never starts, the old one never stops.

Probed: `after removing calm: names = ["combat"], voices still live = 2`; `after retargeting: voices = ["track.a"]` when
the score says `track.b`.

**Runner-up:** the five decision files. All are public-enum questions for the owner.

The pick wins on the loop's first criterion: the audible mix is not the mix the score describes, it is silent, and it
grows worse the longer a project swaps scores.

**Carved off:** quantising the restart onto the bar. A retarget could start on the next beat for musicality, but the
existing `MusicLayers` path already provides `When::NextBeat` for a *gain* change; a cue change today has no quantisation
at all, and inventing one here would need its own decision (what happens to the gap between the fade and the bar?). The
restart is immediate, like `start_layers` itself.

## Design

`src/music.rs`, `start_layers` only (plus its imports).

```rust
// **A layer's voice belongs to the score's layer, and follows it.** Removing or retargeting a layer stops its voice
// through the crate's one stop path; a gain edit leaves the voice alone, because that is what `move_layers` moves.
let mut stops: Vec<(Entity, &'static str)> = Vec::new();
state.layers.retain(|layer| {
    let Some(scored) = score.layer(&layer.name) else {
        if let Some(voice) = layer.voice { stops.push((voice, "layer removed")); }
        return false;
    };
    if scored.cue != layer.cue {
        if let Some(voice) = layer.voice { stops.push((voice, "layer retargeted")); }
        return false;            // dropped; the loop below re-adds it with the new cue
    }
    true
});
for (entity, why) in stops { stop_voice.write(StopVoice { entity }); }
```

`start_layers` gains `MessageWriter<StopVoice>`. It runs in `SirenSystems::Choose`, before `Play` and before `Mix`'s
`stop_requests`, so a stop written here fades the same frame and a start written here is answered the same frame — the
two halves of a score edit land together. `stop_requests` already logs the `Stopped` line with the reason, so this
change adds no second log path.

A re-added layer's `voice` is `None`, so the existing "start every layer with no voice" loop plays the new cue; the
`awaiting` list matches it by cue as it does at startup.

## Every field touched on a serde type

None. No `CueTable`, `SirenSettings` or `MusicScore` field moves.

## Tests that will prove it

`tests/music.rs`:

1. `a_layer_removed_from_the_score_stops_its_voice` — two layers live, the score edited to one; the removed layer's
   voice fades and is gone within `DEFAULT_FADE_S` × 4, one `Stopped { why: "layer removed" }` line, and the surviving
   layer is untouched. **Fails on the unfixed tree** (2 voices live, no line).
2. `a_layer_retargeted_to_another_cue_plays_the_new_one` — one layer on `track.a`, the score pointed at `track.b`; after
   the edit exactly one voice is live and its cue is `track.b`, with a `Stopped { why: "layer retargeted" }` line.
   **Fails on the unfixed tree** (the voice stays `track.a`).
3. `a_layer_whose_gain_changed_keeps_its_voice` — the pair that stops the fix from restarting on every edit: a
   `gain_db` edit leaves the same entity live and does not replay.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (cases 15 and 23 are the score's and must stay green);
- `cargo run --release --example perfcheck` before/after — the reconcile runs only when `MusicScore` is changed;
- `examples/render` and `examples/music`: cue WAVs byte-identical, transitions still on the bar;
- the two probes as tests, and a fresh-eyes run with a score whose layers are added, removed and retargeted in one
  session.

## Risk

A removed layer's voice now fades out over 0.12 s rather than stopping dead, so a project swapping scores hears a short
tail of the old stem. That is the crate's own fade for every stop and is deliberate; the alternative is a click. A
retargeted layer that is **silent** (`SILENT_DB`) restarts its voice for a stem nobody can hear — wasteful but
inaudible, and `MusicScore::problems` already warns when a score's layers are not loops.
