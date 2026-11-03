# review.md — iteration 52, three passes

## Pass 1 — Function: does the report name a duck that lowers nothing?

**The claim** (Gregory, *Game Engine Architecture* 3e §14.6.3): a duck is a **reduction** — −8 dB on music, −4 dB on
effects, hold 0.25 s, fade 0.6 / 0.4 s. A `by_db` of `0.0` is not a gentler duck; it is the absence of one.

**Measured**, `SirenSettings::problems()`, one duck, `max_voices: 32`:

| duck | before | after |
|---|---|---|
| `by_db: 0.0`, hold 0.25, fade 0.6 | **`[]`** | 1 sentence naming `Music` and the `0 dB` |
| `by_db: -0.0`, hold 0.25, fade 0.6 | `[]` | 1 sentence |
| `by_db: 0.0`, no hold, no fade | `[]` | 1 sentence — the depth, not the envelope |
| the crate's `default_ducks()` (−8 dB) | `[]` | **`[]`** — a real duck stays silent |
| `by_db: +6.0` | 1 sentence | 1 sentence, unchanged |

The row that matters as much as the first is the fourth: a report that fired on Gregory's own duck would teach an author
to ignore the block, which is the worst thing a report can do.

**The behaviour is unchanged, proved rather than assumed.** `a_zero_depth_duck_still_fires_and_still_moves_nothing`
plays a `0.0`-dB rule and asserts the bus does not move **and** that the `Ducked` entry is still written. The rule is
inert, not absent — exactly what `active_duck_db`'s `by_db >= 0.0` guard and `duck_db`'s `if db < deepest` already did.

**Verdict: GREAT.**

## Pass 2 — Zoom: the panel, and the fault that would have been invisible

**The first capture would not have proved it.** `SIREN_MIX=broken` already fills the block's three-sentence cap:
`master_db is not a number`, the no-hold sentence, the self-duck, then `and 1 more`. A zero-depth fault added there
would be the hidden fourth.

**`SIREN_MIX=zero` gives it its own table**, following `overbudget`'s precedent, and
`evidence/panel/zero-after.png` shows the whole block as the one sentence:

```
the duck on `Music` lowers it by 0 dB, which is nothing at all; set by_db to the reduction you want
```

One line, no `and N more`, the log intact below it, and the sentence names the field to fix.

**A test I wrote wrongly, caught by running it.** `zero_depth_is_one_sentence_however_it_is_written` asserted a
zero-depth rule with no envelope produces **two** sentences. It produces **one**, because the no-hold sentence is
guarded on `by_db < 0.0` and a zero rule can never reach it. The code was right and my expectation was not; the test now
asserts one sentence and that it does *not* mention `hold_s`.

**Ordering.** The zero-depth check sits **before** the envelope check. For a `0.0` with a hold and a fade the envelope is
real and the depth is wrong, so an author told *"set hold_s or fade_s"* would edit the wrong field.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: is the fault authorable, and does the audio path care?

**Authorability, measured two ways.** Written as RON, `by_db: nan` is **refused by the parser** (`Expected float`), so a
non-finite depth is not a thing an author can type and needs no sentence. `by_db: 0.0` **parses** and reported `[]` —
that is the case this iteration closes. The runtime entry `DuckClock::manual(target, by_db, …)` is where a game computes
a depth from gameplay and can pass zero at the bottom of its range.

**The audio path is untouched.** `render`'s `bus-Music` still reads `duck -8.0 dB, deepest -7.96 dB, recovered by
1.97 s` and `mix.wav` still integrates **−18.6 LUFS at −6.1 dBFS**; the cue spectrogram
(`evidence/spectrograms/grass-spectrum.png`) has a clean attack with no click or clipping edge. That is the expected
result for a change to a report — and it is the check that says so rather than assuming it.

**Can a developer read it without the code?** Yes: the sentence names the bus, the depth and the field.

**Verdict: GREAT.**
