# review.md — iteration 13, three passes

## Pass 1 — function

**The claim.** `max_instances` is a budget on **plays**, not a standing count of the voices sounding, and the docs said
only the latter.

**Against the sources.** The field read *"How many of this cue may sound at once"* — a state. R4 and Gregory §14.5.8.4
both describe **admission**: *"the number of sounds that are permitted to play simultaneously"*. Probed, the two
readings differ exactly where a project is most likely to hit them:

| sequence | observed | documented now |
|---|---|---|
| two voices at `max_instances: 2`, table edited to 1, third play | refused `VoiceLimit`; **both voices still sounding**; nothing stopped, nothing logged | the cap governs the *next* play; live voices live out their length |
| cap raised from 1 to 2 under one live voice, next play | `Played` | unchanged |

**The alternative was rejected with a reason, not laziness.** Reconciling live voices would despawn a voice the game did
not stop, on a frame it did not choose, and log a stop it did not request — three surprises to replace one — and it
needs rules for which voice goes and for a table edited twice in a frame. That is a feature with its own decision, and
it is in the log's next-5.

**The test pins the reading, and the mutation proves it**: removing `room_for`'s unstealable guard (the shape a
"reconcile by ending" implementation would have) reddens `a_cap_lowered_under_live_voices_does_not_end_them`.

**Verdict: GREAT** for a documentation contract with two tests holding it.

## Pass 2 — zoom

**Read:** the probe output, `evidence/{before,after}/`, the cue spectrograms, loudness, and `examples/music`'s bar check.

**Two defects in my own tests, both fixed:** the first version used priority-0 voices, so the lowered cap **stole**
rather than refused and the test asserted the wrong thing; and the second read a `Played` left buffered by the previous
play. Both are the loop's old traps — "a cap that can steal never refuses" and "drain between plays" — and both are
worth their lines in the command.

**Audio unchanged**, as required for a doc change: 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS / −12.1 dBFS looped
past the gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS, and `examples/music` still prints *"transitions on the bar:
yes"* at 4.80 and 7.20 s.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The two tests are a pair — lowered cap refuses and does not cut; raised cap admits — so a
build that stopped admitting plays would be red on the second.

**Can a developer understand it without the code?** `Cue::max_instances` now says *"may **start**"*, gives the reason,
and `CueTable::insert` points at it, which is the method a project calls when it edits a table at runtime. The guide's
voice-budget section says it in a project's words: editing a table under playing sounds is safe.

**What a fresh reader would still ask:** should there be a *way* to reconcile, for a project that wants the count
enforced? Yes, and that is the next-5 item — as a method a game calls, not something the crate does on a frame nobody
chose.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (two test fixtures); fixed.
