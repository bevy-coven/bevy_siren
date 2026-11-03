# review.md — iteration 12, three passes

## Pass 1 — function

**The claim.** A `StopVoice` for a voice the crate has already retired is a no-op, not a panic.

**Against the sources.** `retire_voices` despawns a one-shot the frame it ends — the crate's own schedule. `Played::entity`
is documented as the handle "for `StopVoice`", so the crate hands a game an id and then, on the crate's own retirement,
makes it stale. Bevy 0.20's default error handler **panics** on a command against a despawned entity, and recommends
`queue_handled`/`queue_silenced` for exactly this. The crate's rule is "no `unwrap`/`expect`/`panic!` outside tests"; this
was a reachable panic from a documented workflow.

| sequence | before | after |
|---|---|---|
| play a one-shot, let it end, `StopVoice(handle)` | **panic: `Entity despawned … its index now has generation 1`** | no-op; nothing logged |
| play a loop, `StopVoice(handle)` | fades | unchanged |
| `StopVoice` twice for a fading voice | second stop re-armed the fade | skipped; one fade, one line |
| `StopCue("wind")` with two voices | both stopped | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The evidence is the strongest kind**: the regression test does not merely assert a value, it **panics inside
`app.update()`** on the unfixed tree, with Bevy's own message. That is the difference between "a behaviour changed" and
"a crash stopped".

**Why it was reachable and nobody noticed**: every other `commands.entity(..)` in the crate takes an entity from a live
query, which cannot yield a dead one. This was the only path where the entity came from a **message** — the crate's own
id, checked; Bevy's id, not.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** the panic output, the fixed tree's test output, `evidence/{before,after}/render.txt`, the cue spectrograms and
loudness.

**Two defects in my own work, both fixed:** a leftover `StdArc` import and a stray expression in the first test (a
scaffold I typed and did not delete); and the test's use of `Entity::from_raw_u32(...).unwrap_or(...)` in the probe, which
was reaching for an id instead of using a real despawn — the test that shipped despawns the actual voice, which is the
path a game hits.

**Audio is byte-identical**, as required: nothing a play decides moved. 16/16 WAVs, `ui-cancel` −24.4 LUFS / −12.1 dBFS
looped past the gating block, `mix.wav` −4.3 LUFS / +0.2 dBFS.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** Three tests: the stale handle (the crash), a second stop (the fade begun once), and
`StopCue` (the name path, which must not narrow). The third exists so a guard that broke the normal stop would be red.

**Can a developer understand it without the code?** `StopVoice`'s doc should say what a stale handle means; the code
comment does, and the `CHANGELOG` states it as a game's ordinary race rather than a mistake. That is the sentence a host
needs.

**What a fresh reader would still ask:** should a stale stop leave a **log line**? The log holds decisions about sounds;
a stop for a sound that stopped is not a decision, and the answer is that there is nothing to record. If the owner wants
one, it belongs with the `Entry` decision three other files wait on.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 1 (the guard's placement); it is in the one loop that
needed it.
