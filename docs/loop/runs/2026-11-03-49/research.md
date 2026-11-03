# research.md — iteration 49: a bus can sum past full scale, and the crate could not say so

## The item

`examples/render` has written a line for several iterations:

```
voices-8: peak 3.419, rms 0.397, 307 samples would clip (the budget's job, not the gain's)
```

**The parenthetical is false.** `max_voices` is a **sample-cost** budget — `docs/research/siren.md`: *"more than a busy
scene's close effects, fewer than a mixer's worth of sample cost"* — and it does not, and never claimed to, bound the sum
of what is playing. Eight sources at 0 dB sum to **+18.06 dB**, and the file is written with 307 samples at the rail,
measured from the WAV itself:

```
voices-8: 8820 samples, peak 1.000 of full scale, 307 at the rail (clipped)
```

The crate's own R6 says the mechanism plainly — *"several sounds **sum in linear** (two 0 dB sounds are one at
+6.02 dB)"* — and `model::sum_db` already computes it. But **nothing in the crate let a game ask the question**, so even
the harness's author assumed the budget covered it.

**And the example's own shipped table is over the rail**: with two voices live, `Effects` reaches **+6.0 dBFS** and
`Music` **+4.0 dBFS**. That had never been on screen.

## Sources

**S1. `docs/research/siren.md` R6** — *"several sounds **sum in linear** (two 0 dB sounds are one at +6.02 dB)"*. The
arithmetic, stated as a crate rule since the first iteration.

**S2. `docs/research/siren.md`, `max_voices`** — *"more than a busy scene's close effects, fewer than a mixer's worth of
sample cost"*. A **sample-cost** budget; the word "headroom" does not appear.

**S3. `model::sum_db`** — already exists, already used by the example, already the right function. Nothing in `src/`
but the example called it.

**S4. `SirenMix`** — carries each bus's gain and voice count, which is exactly the two numbers the bound needs. It had
`bus_db`, `voices_on`, `deepest_duck`, and nothing that combined them.

**S5. `docs/loop/needs-decision/README.md`** — a **new method** on an existing public type is additive; no `match`
breaks, and nothing is removed. Not a decision.

## The technique

`SirenMix::worst_peak_db(bus) -> f32`: `n` live voices, each at full scale (0 dB), plus the bus's own gain (master and
ducks included), summed with `model::sum_db`. An **upper bound**, not a measurement — the crate mixes samples and never
reads their amplitude, so it cannot know what a recording peaks at, and the doc says so.

`SILENT_DB` for a bus with no voices: "nothing playing" is not a peak, and reporting `−80` in another sense would read as
a bus mixed into the floor.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| two 0 dB sounds sum to | **+6.02 dB** | S1 |
| eight 0 dB sounds sum to | **+18.06 dB** | S1, arithmetic |
| `max_voices`'s own claim | a sample cost, never headroom | S2 |
| two voices at full scale on a −6 dB bus | **+0.02 dBFS** | measured, in the doctest and a test |
| a bus with no voices | reads `SILENT_DB` | **design default** |
| the example's table, two voices | `Effects` **+6.0**, `Music` **+4.0 dBFS** | measured on the captured panel |

## What good sounds like

A `Music` block whose tail reads `peak Effects +6.0 dBFS / 2  Music +4.0 dBFS / 2` — the two buses the shipped table
takes over the rail, with the voice count that takes it there — and no such clause on a bus that is not at risk. A
`Problems` block that still says `none`, because the table is not wrong: the sum is a property of what is playing.
