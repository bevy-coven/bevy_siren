# review.md — iteration 33, three passes

## Pass 1 — function

**The claim.** `When::NextBar` is *"on the next bar, quantised with `next_boundary`"*. `move_layers` computed that instant
**once**, at request time, from the tempo then; `Pending` stored only `at_s`. A tempo edit while a change waits left it
scheduled against a bar that no longer exists.

| case | source | before | after |
|---|---|---|---|
| queued `NextBar` at 160 bpm, score → **80** | S1 (`NextBar`'s doc), S2 (the guide: *"layers that change on the bar"*), S3 (`MusicScore` is *"the Cauldron edits"* it) | fired at **1.44 s** — a bar at 80 is 3.0, so **mid-bar** | fired at **2.944 s** — on the bar |
| queued at 160, score → **240** | S1 | fired at **1.44 s** — a bar at 240 is 1.0, so **a bar late** | fired at **0.944 s** — on the bar |
| tempo unchanged, 160 / 80 (guards) | S4 (`next_boundary` is right; the caller froze its answer) | 1.44 / 2.944 | **identical** |
| `When::Now` | S3 | applies on its own frame | **unchanged** |

**Against the sources.** S3 is the clincher: `MusicScore` is a resource the Cauldron edits, so a tempo change *while a
change waits* is the ordinary path, and iteration 14 fixed exactly this class for **layers** — `start_layers` reconciles a
changed score — while the **pending queue** kept its old tempo. The failure is in both directions: too early after a
slowdown, too late after a speed-up.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/tempo{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**The measurement is by firing, not by inspection.** The probe walks the clock until `target_db` moves, so the number is
*when the crossfade actually happens* rather than what `next_change_s` reports — the two could differ if the due-filter
disagreed with the queue. They agree: `2.944` against a 3.0 s bar, `0.944` against 1.0 s, `1.44` against 1.5 s — each within
one 16 ms frame, which is the tick the probe advances by.

**A guard that would have caught a naive fix, written before the fix was trusted:** `a_queued_change_holds_at_an_unchanged_tempo`
asserts the instant is **exactly** unchanged after thirty frames. A re-quantise without the `tempo` guard would push the
change forward every frame and never fire it.

**Audio unchanged.** 21/21 WAVs byte-identical; `mix.wav` −4.3 LUFS / +0.2 dBFS. The scripted captures never retempo, so
every rendered cue is untouched — which is the point: this is a state fix, not a sound change.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** Four runs of one probe: two with the tempo unchanged (the baseline each new tempo should produce),
and two with the tempo moved in **opposite directions** — a slowdown and a speed-up — because one direction only would
leave the other's sign error untested.

**Can a developer understand it without the code?** `Pending`'s two new fields carry a doc comment naming the measured
failure (*"a change queued for the next bar at 160 bpm and fired at 1.5 s is the middle of a bar at 80 bpm"*), and the
re-quantise block says why `at.max(now)` and why the `tempo` guard exists.

**What a fresh reader would still ask:** *why not recompute every frame?* Because a per-frame re-quantise moves a queued
change forward for ever — the guard test pins that. The tempo is the only input worth noticing.

**Verdict: GREAT.**
