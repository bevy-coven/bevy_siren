# review.md — iteration 14, three passes

## Pass 1 — function

**The claim.** A `MusicScore` edited at runtime reconciles its **voices**, not only its `MusicState` entries: a removed
layer's stem stops, and a retargeted layer plays the new cue.

**Against the sources.** `MusicScore` is documented as *"a file a project ships and the Cauldron edits"* — a value that
changes at runtime. `start_layers`' own doc says *"one looping voice per score layer"*; a layer the score no longer lists
is not one. R5's layering is *"stems mixed in and out according to game state"*: which stems are **running** is part of
game state, not only their gains. And `retire_voices` skips loops by design, so an ownerless loop plays until the
process ends — which is why the system that orphans it must stop it.

| edit | before (probed) | after |
|---|---|---|
| remove layer `combat` | `names = ["calm"]`, **2 voices live**, no line | `names = ["calm"]`, **1 voice**, `stopped track.combat: layer removed` |
| retarget `calm` → `track.combat` | **the old cue plays**, the new never starts | only `track.combat` is live; `stopped track.calm: layer retargeted` |
| change a layer's gain | eases | unchanged, same entity |
| add a layer | starts | unchanged |
| every rendered cue, the mix, `examples/music` | — | **byte-identical**; *"transitions on the bar: yes"* |

**A third defect the first fix introduced, found by the gate:** my reconcile dropped a retargeted layer and re-added it at
the end, so `MusicState::names()` — documented as **score order** — came back as `["combat", "calm"]`. Fixed by sorting
the live layers to the score's order after the reconcile. A fix that breaks a documented ordering while fixing a
documented agreement is not a fix.

**Verdict: GREAT.** The score and the sound agree after every edit, and each row is a test.

## Pass 2 — zoom

**Read:** the two probes, the after-tree output, the cue spectrograms, loudness, and `examples/music`.

**A wrong turn, corrected:** I first sent a `StopVoice` and let `stop_requests` log the line — which logs every stop as
`"stop"`, because a `StopVoice` carries an entity and no reason. The test caught it (`stopped track.combat: stop`). The
fix marks the voice `Stopping` directly with the true reason, which keeps the one fade and one log line and leaves the
message unchanged — a message change would have been a decision file for a word.

**Audio unchanged** where nothing was edited: 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS / −12.1 dBFS looped past
the gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS, music transitions still on the bar at 4.80 and 7.20 s.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** Three tests: removal, retargeting, and a gain edit that must **not** restart — the pair
that stops the fix from turning every score edit into a phrase restart, which would be audible.

**Can a developer understand it without the code?** The guide now says a score's layers are voices that follow the
score, and the two new log reasons name what happened. `MusicScore`'s doc and `names()`'s score-order promise both still
hold, and the second is now tested by the very change that could have broken it.

**What a fresh reader would still ask:** should a retarget start on the bar? `When::NextBeat` exists for a *gain* change;
a cue change has no quantisation today, and inventing one needs a decision about the gap between the fade and the bar.
Carved off and named.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 1 (the ordering regression); fixed.
