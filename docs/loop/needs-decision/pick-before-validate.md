# Should a refused play advance the variant window and spend a draw?

**Status:** open — found in iteration 59 by probing; the behaviour is **protective** and was kept, not changed.

## What the crate does now

`answer` picks the variant **before** it validates anything else:

```rust
let Some(variant) = choices.pick(cue) else { return Err(Refusal::NoSounds) };   // <- draws + advances
let Some(handle) = sounds.get(&slot.sound) else { return Err(Refusal::MissingSound { .. }) };
```

So a play refused for a **missing sound**, an **unknown bus** or a **missing listener** has already drawn from
`SirenChoices::rng` and advanced that cue's no-repeat window.

**Measured** (iteration 59, `Repeat::Shuffle`, two sounds, seed default):

| app | handles | sounds that played over 8 requests |
|---|---|---|
| `full` | `a`, `b` | `b a b a b a b a` |
| `half` | `a` only | `a a` — and its second `a` is the third of `full`'s stream |

The refusal spent a draw. That looks wrong, and it is not:

**Measured** (`Repeat::AvoidRepeat { avoid: 1 }`, `a` present, `b` missing, 8 requests): **4 of 8 plays sounded**. If the
picker did **not** advance on a refusal it would keep re-picking `b`, refuse, and re-pick `b` — a cue that never sounds.
Advancing is what lets a cue with one missing file still play its other variant.

And the two are inseparable: `VariantPicker::pick_from` draws and advances in one call, so "spend the draw but not the
window" is not a thing the current API can express.

## The options

### A — keep it, and say so in the guide

- **Cost:** none. A project that measures its own RNG stream sees refusals consume numbers.
- **Benefit:** a cue with a missing file still sounds half the time, which is what a player needs.
- **This is what the crate does today.** The gap is only documentation: nothing says a refusal touches the stream.

### B — pick after validation

Validate the cue, the bus and the handle **before** calling `pick`, so a refusal touches nothing.

- **Cost:** a **behaviour change**, and the livelock above becomes real: a cue whose first choice is a missing file
  would refuse on every request, because the picker never advances past it. The fix would need a second rule — "advance
  the window but do not draw", or "skip a slot with no handle" — and `pick_from` cannot express either without a new
  parameter.
- **Benefit:** a project's stream is exactly its sounding plays.

### C — validate the handle first, then pick among the slots that *have* handles

- **Cost:** a new parameter on `pick_from` (the set of playable slots), so a real API change; and the picker's window
  would then be over a *smaller* set than the cue's sounds, which changes what `avoid: n` means.
- **Benefit:** no refusal for `MissingSound` is possible at all — the crate would stop picking a sound it cannot play,
  which is arguably the honest answer.

## The recommendation

**A now**, with a guide sentence. The behaviour is protective, proved by the 4-of-8 measurement, and B and C both need
a rule the current picker cannot express. A project that wants the strict stream (C) is asking for a different feature —
*"never pick a sound I cannot play"* — which is worth its own decision, not a bug fix.

## What is blocked

Nothing. The crate keeps picking before validating, and `tests/play.rs::a_cue_with_a_missing_file_still_sounds` is the
proof that this is the behaviour worth keeping.
