# review.md — iteration 9, three passes

## Pass 1 — function

**The claim.** A stinger's duck is armed when its `Play` is **answered `Played`**, not when the request is written; a
stinger that cannot play leaves the music untouched and never logs a duck.

**Against the sources.** Gregory §14.6.3 and the crate's own R3 both say a duck is driven by **a voice that starts**. The
dialogue path already works that way (`clock.fire_all` inside `play_requests`); the stinger path was the one place that
armed a duck without a voice. The crate's guide says a stinger *"ducks the `Music` bus **while it runs**"*.

| case | before | after |
|---|---|---|
| cue not in the table | **Music −9.00 dB**, `Refused::UnknownCue`, log says it ducked | Music 0.00 dB, `Refused::UnknownCue`, **one** line |
| cue on `Music`, no handle for its sound | **Music −9.00 dB**, `MissingSound`, log says **both** ducked and refused | Music 0.00 dB, only the refusal |
| a stinger that plays | −9.0 dB on the frame it fires, one `Ducked` | unchanged |
| a stinger on another bus | refused, no duck | unchanged |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The second row is the clearest evidence**, and it came from the log itself: on adjacent lines, *"stinger ducked Music
by -9.0 dB"* and *"refused … no handle …"*. Two records of one request disagreeing is the failure mode the crate's
refusals exist to prevent.

**Two independent halves, both proven.** The table lookup (a cue with no bus cannot pass a bus check) and the
ordering (a duck waits for `Played`) each fail the corresponding test when reverted alone — checked, and the first
revert left one test passing because the other half still refused it.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** the four new tests' output, `evidence/{before,after}/refusal-*.txt`, the cue spectrograms and loudness.

**One defect found and fixed in my own work:** the wrong-bus branch briefly borrowed `Refusal::UnknownBus { bus }`, which
would print *"bus `Effects` is not in the mix table"* — **false**, since `Effects` is in the table. Reverted to the true
prose line in `SirenLog`, and `needs-decision/stinger-refusal.md` carries the variant that would say it properly. This
is the same class the last three iterations fixed, and it would have been a regression introduced by the fix.

**A second decision taken during the pass:** stinger refusals now write a `Refused` **message**, not only a log line —
`UnknownCue` for the unknown cue, which is exactly true. A host and the panel read messages; a refusal only in the log
is half a refusal.

**Audio unchanged:** 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS / −12.1 dBFS looped past the gating block,
`mix.wav` −4.3 LUFS / +0.2 dBFS, the attack crop continuous at 35 ms.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** Four tests cover the matrix: unknown cue, missing sound, a real stinger, wrong bus. The
positive case asserts the dip **on the frame it fires** — the thing `before(SirenSystems::Mix)` buys — so a rewrite that
merely worked but landed a frame late would be red.

**Can a developer understand it without the code?** The `CHANGELOG` states the ordering in one sentence and names the
decision file for the remaining gap. The log now says one thing per stinger: either `Ducked` or `Refused`.

**What a fresh reader would still ask:** why a wrong-bus stinger has no `Refusal` variant. The answer is in
`needs-decision/stinger-refusal.md` rather than implied: the honest variant is new, and borrowing one would print a
false sentence.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a false sentence I nearly shipped); fixed.
