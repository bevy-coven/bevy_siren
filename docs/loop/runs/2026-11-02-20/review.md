# review.md — iteration 20, three passes

## Pass 1 — function

**The claim.** `SirenSettings::problems()` is what a host and the Cauldron's panel read to find a mix-table mistake, and it
disagreed with the mix about what a bus is.

| table | source | before | after |
|---|---|---|---|
| `buses: []` (the crate's four, documented) | S1 (`docs/guide/buses.md`: *"everywhere — for a play's bus check, for the panel, and for the mix"*), S2 (`bus_names()`) | **4 sentences**, all false: `Dialogue`, `Music`, `Effects`, `Dialogue` "which the table does not have" | **0 sentences** |
| two buses (a menu mix) | S3 (a fault is named once) | **2 identical** sentences for `Dialogue` | **1** |
| one bus + two rules naming `Ghost` | S3 | **2 identical** sentences | **1**, with the untouched words |
| default table | — | 0 | 0 (guard) |

**Against the sources.** S2 is the whole point: `bus_names()` is the one implementation of S1, and `problems()` built its
own `BTreeSet` instead — the same bypass iteration 10 closed in `update_mix`. The crate's own trap list is about exactly
this shape.

**The severity:** a report that invents four faults for a table the crate **mixes correctly** is worse than a missing
report, because it teaches an author to ignore the panel. That is why this outranked the waiting `Refusal` variants.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `spectrograms/{ui-cancel,footstep-grass}-{spectrum,attack}.png`, the loudness table, both probe files.

**Audio unchanged, deliberately.** 21/21 WAVs byte-identical; `ui-cancel` −24.4 LUFS / −12.1 dBFS,
`footstep-grass` −18.4 LUFS / −6.1 dBFS (looped), `mix.wav` −4.3 LUFS / +0.2 dBFS — identical to iterations 14–19. The
report does not touch the mix, so a changed WAV would mean the fix had gone somewhere it should not.

**The measured table**, `evidence/report{,-before}.txt`, four tables:

```
before                                                    after
default table                0 sentences                   0
buses: []                    4, all false                  0
two buses                    2 identical for Dialogue     1
one bus + Ghost x2           2 identical for Ghost        1
```

**A defect in my own fix, caught here — the only one that mattered:** my first version deduplicated by **bus name**, so
`Nope → Music` and `Music → Nope` (two genuinely different faults about one absent bus) collapsed into one sentence and
the crate's **pre-existing** test caught it. The key has to be the **sentence**, not the bus: identical sentences are one
fault, different sentences about one bus are two. Without the old test this would have shipped a report that under-counts.

**Verdict: GREAT** after that correction (pass 1 re-ran).

## Pass 3 — fresh eyes

**A different shape.** Four tables, none of them the default shape alone: the empty list, a two-bus menu, a one-bus table
with two rules naming one absent bus, and the default as the guard. The "must not happen" half
(`a_duck_naming_a_missing_bus_is_still_reported`) asserts the **exact sentence**, so a fix that stopped reporting cannot
pass by saying nothing.

**Can a developer understand it without the code?** `docs/guide/buses.md` already said *"everywhere"*; the guide gains
the sentence that the report is one of those places. A panel printing `problems()` now prints what the mix believes.

**What a fresh reader would still ask:** *why keep both sentences for `Nope`?* Because one says a duck **starts** on a
missing bus and the other that it **lowers** one — a designer fixes them differently, and a report is for a designer.

**Verdict: GREAT.**
