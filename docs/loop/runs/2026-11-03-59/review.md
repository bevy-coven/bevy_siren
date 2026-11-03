# review.md — iteration 59, three passes

## Pass 1 — Function: does an empty bus differ from a silent one?

**The claim is the method's own doc** (`SirenMix::worst_peak_db`): *"Nothing playing is not `SILENT_DB` — it is **no peak
at all**, and `SILENT_DB` would read as a bus mixed into the floor rather than a bus with nothing on it."*

**Measured**, `SirenMix` built by hand:

| | `worst_peak_db` | `peak_db` |
|---|---|---|
| a bus with **no voices** | `SILENT_DB` (−80.0) | **`None`** |
| 2 voices on a bus **mixed to the floor** | `SILENT_DB` (−80.0) | `Some(−80.0)` |
| 2 voices at −70 dB | −64.0 | `Some(−64.0)` |

The first two rows are the collision: `assert_ne!` between them **fails**. The method's doc described a distinction its
body did not make, and `peak_db` now makes it — `None` for absence, `Some` for a real value including one at the floor.

**Verdict: GREAT.**

## Pass 2 — Zoom: additive, and who was working around it

**`worst_peak_db` was not changed.** It is the one-number form, and a caller that wants a number for every bus has it.
Changing its signature to `Option<f32>` is the natural API — and the wrong move on a **published** crate (0.1.0, nine
downloads): every `mix.worst_peak_db(bus) > x` would stop compiling for a naming nit.

**The workaround was in the crate's own example.** `examples/mixer` filtered `voices_on(bus) > 0` **before** asking for the
peak — exactly the discipline the sentinel forces on every caller, and the reason this is worth an iteration even though
no caller is currently wrong. That filter is gone; the panel now uses `peak_db` and `filter(…)`, which says the same
thing in the API's own vocabulary.

**Ranked honestly.** This sits below the blocked report items because **no caller in this repository is wrong today**.
It is a latent trap: a game writing `worst_peak_db(bus) > -1.0` gets `true` for an empty bus — right by luck, and
indistinguishable from a deliberate mix at the floor.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: a bug that was not one, and the panel

**A finding that was *not* a bug, and is now a decision file.** I hypothesised that a **refused** play should not advance
the variant window: `answer` calls `choices.pick(cue)` before it validates the handle, so a `MissingSound` refusal has
already drawn from the RNG and moved the no-repeat window. Measured with two sounds and one handle missing: the sounding
streams differ between a full app and a half app.

Then I tested the opposite hypothesis, which is the one that decides it: with `avoid: 1` and one file missing, **4 of 8
requests still sound**. If the picker did *not* advance on a refusal it would re-pick the missing sound forever — a cue
that never sounds. And the two cannot be separated: `pick_from` draws and advances in one call.

So the behaviour is **protective**, not buggy. It is recorded in `needs-decision/pick-before-validate.md` with both
measurements, so a later iteration does not "fix" it into a livelock. That is the more valuable output of the probe than
a patch would have been.

**The panel.** `evidence/panel/panel.png` renders with the new method: bus levels, the music block, the cue list, `Problems:
none`, and the log tail — no peak clause, because every bus has headroom now. The screenshot is unchanged from before,
which is the claim: **a caller's picture does not move when a sentinel is replaced by a type that cannot collide**.

**Nothing audible moved**: `bus-Music` still `duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`; `mix.wav` still −18.6
LUFS at −6.1 dBFS; the cue spectrogram is a clean broadband transient with no clipping shelf.

**Verdict: GREAT.**
