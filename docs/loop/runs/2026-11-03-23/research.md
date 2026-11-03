# research.md — iteration 23: a seed the crate reports but the stream did not come from

## The item

`SirenSettings::seed` is what a project writes to make a capture, a replay or a test reproducible.
`SirenSettings::default_seed()` is **`0x5EED_51AE_0000_0001`**. `Rng::new(0)` replaces a zero seed with **that same
constant** — correctly, because xorshift's state must never be zero — but `SirenChoices` records the seed it was
**asked** for, not the one it used:

```
SirenSettings { seed: 0 }        ->  SirenChoices::seed() reports 0
SirenSettings::default()         ->  SirenChoices::seed() reports 0x5EED51AE00000001
both streams                     ->  ["k2", "k0", "k2", "k1", "k3", "k0"]   identical
seed 1                           ->  ["k0", "k2", "k1", "k2", "k1", "k0"]   different
```

So `seed: 0` and the default seed are **one stream wearing two names**, and `seed()` reports a number the stream did not
come from. `SirenChoices::seed()`'s doc says *"the seed this stream was built from"* — under `0` that sentence is false.

## Sources

**S1. `Rng`'s own doc** (`src/model.rs`) — *"A game that wants its sounds reproducible … seeds this and gets the same
sequence; one that does not still gets a sequence, because a `0` seed is mapped to a constant. Never seeded from the
clock inside this crate: **a sound that cannot be reproduced cannot be debugged**."* The doc is right about the mapping
and does not say that the mapping makes `0` **the default seed's stream**.

**S2. `SirenChoices::seed()`'s doc** — *"The seed this stream was built from."* This is the sentence the bug falsifies: the
stream was built from `0x5EED…`, and the method reports `0`.

**S3. `docs/loop/cues.yaml` case 17** — *"The same seed replays the same sounds, gains and pitches; two seeds are two
streams. Equal field for field within a seed, and different across seeds."* Read literally, `seed: 0` and
`seed: default_seed()` are two seeds and must be two streams. They are not.

**S4. `Rng`'s implementation, held against its doc** — the doc says *"xorshift64\*"*; the code is `x ^= x << 13; x ^= x
>> 7; x ^= x << 17;` with **no** final multiply, which is Marsaglia's plain xorshift64, not the starred variant. Measured,
the stream is uniform and long, so this is a **naming** fault, not a quality fault: 400 000 draws over four bins gave
99754 / 99824 / 100469 / 99953, and the 8-draw window did not repeat in 20 000 000 draws.

## The technique

Two changes, neither touching a public type:

1. **`SirenChoices` records the seed the `Rng` was actually built from.** `Rng` gains `seed()` — the effective seed, `0`
   mapped to the constant — so there is **one** definition of "which seed is this stream", and `SirenChoices::seed()`
   returns it. A project that writes `seed: 0` is told `0x5EED_51AE_0000_0001`, which is the truth about its stream.
2. **The doc says the mapping.** `Rng`'s doc already explains the zero-state rule; it now also says what `seed()` answers,
   so `SirenChoices::seed()`'s sentence is true by construction.

**Why not make `0` a distinct stream?** Because `Rng(0)` must not be zero-state — that is the rule the mapping exists for.
The honest fix is to report the mapping, not to hide it: a project that wants a stream different from the default writes
**any** seed but `0`, and the crate says so.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a zero seed is mapped to a constant | `0x5EED_51AE_0000_0001` | S1, `Rng::new` |
| the reported seed must be the one the stream came from | — | S2 |
| two seeds are two streams | — | S3 |
| a 4-bin histogram over 400 000 draws | 99754 / 99824 / 100469 / 99953 | measured |
| the 8-draw window's period | > 20 000 000 | measured |

## What good sounds like

A project that writes `seed: 0` hears the crate's default stream **and is told so**: `SirenChoices::seed()` returns
`0x5EED_51AE_0000_0001`, the number `SirenSettings::default_seed()` returns, so a log line, a panel and a test all agree
with the sound. A project that writes `seed: 1` hears a different stream and is told `1`. Nothing about any sound changes.
