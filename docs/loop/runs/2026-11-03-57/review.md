# review.md — iteration 57, three passes

## Pass 1 — Function: does a stinger that cannot dip say nothing?

**The claim** (`Stinger::new`'s own doc): *"A stinger of `cue` ducking the music by **6 dB** for a second."* Its name for
itself is a reduction, and `active_duck_db` returns `0.0` for any `by_db >= 0.0`.

**Measured**, `Stinger { cue: "sting", duck_db: 0.0, duck_s: 1.0 }`:

| | log | `Music` |
|---|---|---|
| before | **`[Ducked { by_db: 0.0, by: "stinger" }]`** | **unmoved** |
| after | **`[]`** | unmoved |

The log claimed a duck the mix did not make. A `Stinger` is a **message**, so `MusicScore::problems` cannot see it — the
log was the only place a person could learn what it did, and it said the opposite.

**The non-regression holds.** −9.0 logs `[-9.0]` (the stinger's own depth) and `Music` reads below −0.5 dB; a `+6.0`
raise logs nothing, because `active_duck_db` treats it as a lift and a lift is not a duck.

**Verdict: GREAT.**

## Pass 2 — Zoom: the rule, and what I threw away

**The condition is the same one the envelope uses**: `duck_db.is_finite() && duck_db < 0.0`, which is `active_duck_db`'s
own `by_db >= 0.0` guard read from the other side. The log and the envelope now agree by construction rather than by
coincidence.

**The stinger's hit still plays.** Only the duck is conditional. A zero-depth stinger is a sound with no dip — a
legitimate choice — and silence about it is correct because nothing lied.

**Nine hypotheses were probed and passed**, and the four cue-cap probes were **deleted** rather than kept: they asserted
behaviour no change touched. My removal script cut mid-comment and left four orphaned doc fragments in `tests/play.rs`,
which failed to parse (`unknown start of token: \``). Repaired by anchoring each cut on the next block's header; the file
now compiles and the three stinger tests remain.

**Two clippy errors the deletion created** — an unused `advance`/`live_voices` import and a non-snake-case test name
(`…_zero_dB_…`) — caught by the gate, both fixed. This is the second iteration running where deleting a test broke a
lint; the lesson is to re-run `clippy --all-targets` after any test removal, not only after additions.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer see it, and did anything else move?

**The panel cannot show it.** `examples/mixer`'s `Music` block draws each layer's `dB -> target` and a `waiting` marker;
a stinger's dip appears there for its hold, but the *absence* of a dip is not drawable — a capture of the fix would show
an unchanged panel. It names its test instead: `a_stinger_at_zero_db_logs_no_duck`.

**Nothing else moved.** `render.txt` still reads `bus-Music: duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s`, and
`mix.wav` still integrates **−18.6 LUFS at −6.1 dBFS**. The stinger spectrogram
(`evidence/spectrograms/stinger-spectrum.png`) is a clean transient at 0 with no clipping edge.

**Can a developer read it without the code?** Yes — the log now has the property it should have had: a `Ducked` line
means the bus moved.

**Verdict: GREAT.**
