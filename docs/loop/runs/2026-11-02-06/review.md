# review.md — iteration 6, three passes

## Pass 1 — function

**The claim.** A `MissingSound` refusal names the **sound key**, never the cue, from both of its causes.

**Against the sources.** `MissingSound::sound` is documented *"the key with no handle"*, and Bevy's own asset model
(`bevy_asset-0.20.1/src/server/mod.rs:2114`) keeps a file that was never found apart from bytes that failed to load,
carrying the error in the second case. The crate had the key in hand on both paths (`answer` chose it; `start_pending`
can recover it from cue + variant) and put the cue's name there on one of them.

| cause | before | after |
|---|---|---|
| a key with no handle | ``no handle for sound `step_03` `` | ``no handle or undecodable bytes for sound `step_03` `` |
| a key whose bytes do not decode | ``no handle for sound `footstep.grass` `` | ``no handle or undecodable bytes for sound `step_03` `` |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The probe pair is the evidence**, and either line alone would pass vacuously: before, the two causes named **two
different subjects** (`step_03` and `footstep.grass`); after, both name `step_03`. The failing-before test asserts the
exact payload: `MissingSound { sound: "footstep.grass" }` on the unfixed tree, `{ sound: "step_03" }` after.

**One thing this does not do.** The sentence is now true for both causes but does not *distinguish* them, because the
variant carries only the key. `needs-decision/undecodable-sound.md` carries the variant that would, with what it costs
an exhaustive `match`.

**Verdict: GREAT.** The key is correct from both paths with no public type change, the non-regressing cause is pinned
beside the fixed one, and the limitation is named rather than papered over.

## Pass 2 — zoom

**Read:** `evidence/{before,after}/missing-sound.txt`, the cue spectrograms and the loudness numbers.

**One defect in my own work, found and dropped:** a third test tried to make a voice wait for a default (unresolved)
handle and then resolve it, so `start_pending` would find the cue gone from the table. `Assets::insert` on a default
handle's id does not make `assets.get(&that_handle)` succeed, so the voice never reached the code under test — the test
proved nothing and would have proved nothing on any tree. Removed rather than contorted; the branch it covered (a table
edited between a play and its load) is narrow and is named in the code's comment instead.

**The spectrograms and loudness are unchanged**, which is the point: no sample, gain or duck moved. `ui-cancel` reads
−24.4 LUFS / −12.1 dBFS looped past the 0.4 s gating block; `mix.wav` −4.3 LUFS / +0.2 dBFS; `footstep.grass` −18.4 LUFS
/ −6.1 dBFS. The attack crop shows a continuous decay at 35 ms, no vertical edge.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The probe runs both causes in separate apps with the same cue table, so the pair is
comparable; the unit tests reach them by their own routes (empty `SirenSounds` vs a refusing `AudioOutput`). The fake
output's new `refuses_samples` flag is the same `None` the real mixer returns when `rodio::Decoder::new` errors, so the
test exercises the crate's path rather than a hand-made branch.

**Can a developer understand it without the code?** `missing-sound.txt` is two lines, one per cause, each a payload
and a sentence. A reviewer sees that both now name `step_03` and that the before tree named the cue on one of them.
The `CHANGELOG` says the same in one sentence and names the decision file for the finer distinction.

**What a fresh reader would still ask:** *which* of the two was it, for a given line? The payload cannot say, and the
honest answer is in the decision file rather than implied by the code.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a test that proved nothing); removed.
