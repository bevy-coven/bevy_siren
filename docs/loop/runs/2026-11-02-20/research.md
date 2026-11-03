# research.md — iteration 20: the report disagrees with the mix about what a bus is

## The item

`SirenSettings::problems()` is the one thing a host and the Cauldron's panel read to find an authoring mistake in the mix
table. Two things are wrong with it, both measured:

```
buses: [Music, Effects]   (the two-bus table of a menu mix)
  "a duck starts on bus `Dialogue`, which the table does not have"
  "a duck starts on bus `Dialogue`, which the table does not have"    <- the same sentence twice

buses: []                 (documented: the crate's four)
  "a duck starts on bus `Dialogue`, which the table does not have"
  "a duck lowers bus `Music`, which the table does not have"          <- but Music IS a bus
  "a duck starts on bus `Dialogue`, which the table does not have"
  "a duck lowers bus `Effects`, which the table does not have"        <- and so is Effects
```

**The second is the serious one.** `docs/guide/buses.md` says an empty list means the crate's four *"everywhere — for a
play's bus check, for the panel, and for the mix"*, and `SirenSettings::bus_names()` implements exactly that. `problems()`
does not use it: it builds its own set from `self.buses`, bypassing the one answer every other reader uses. So a table the
crate **accepts and mixes correctly** is reported as **four mistakes that are not mistakes** — which is the worse failure
for a report, because an author learns to ignore it.

**The first is a duplicate**, and it is the default table's own doing: `default_ducks()` is
`Dialogue → Music` and `Dialogue → Effects`, so `Dialogue` is the `source` of both rules and the same sentence is
pushed twice. A report that names one fault twice reads as two faults.

## Sources

**S1. `docs/guide/buses.md`** — *"A table that lists no buses means the crate's four (`DEFAULT_BUSES`) **everywhere** —
for a play's bus check, for the panel, and for the mix — so a cue the crate accepts always has a row to be mixed on."*
The word **everywhere** is the contract; `problems()` is the panel's source and is not in the set.

**S2. `SirenSettings::bus_names()`** — the one implementation of S1, and the one `answer`'s `UnknownBus` check reads. A
second way to answer "is this a bus" is a second rule, and the crate's own trap list is about exactly that (*"a second
spawn path is a second set of rules"*). Iteration 10 fixed the same shape in `update_mix`.

**S3. R6 (vendored: `docs/research/siren.md`)** — *"everything a play decides is **readable**"*, and the crate's rule that
a fault is named **once**: a panel prints `problems()` verbatim, so a duplicated sentence is a panel that lies about
how many faults a table has.

**S4. Simon Franco, *Creating Dynamic Soundscapes Using an Artificial Sound Designer*, Game AI Pro ch. 46** (vendored:
`siren.md` S2) — the system decides from the facts it is given, and a designer acts on what the system reports. A report
that names a non-fault costs the designer the same as a fault that is not named: attention.

## The technique

1. **`problems()` asks `bus_names()`**, so the empty-list rule has one implementation and the report and the mix agree by
   construction rather than by a comment in two places.
2. **A fault is reported once per bus.** The duck loop already has to check a `source` and a `target`; a bus named by
   two rules — which is the *ordinary* case, because the default ducks share `Dialogue` — must not produce two identical
   sentences. A `BTreeSet` of already-named buses, in the shape the table loop already uses for duplicate bus names.

**The sentences themselves are untouched.** The words are right; only *which* buses they are about, and *how many times*
they appear, change.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| no buses listed means the crate's four, everywhere | `DEFAULT_BUSES` | S1, S2 |
| one bus, one answer: `bus_names()` is the only implementation | — | S2 |
| a report names a fault once | — | S3 |
| a report that cries wolf costs a designer attention | — | S4 |

## What good sounds like

A table with `buses: []` reports **nothing** about buses, because there is nothing wrong with it: the crate's four are
there and the default ducks name buses that exist. A two-bus menu table reports **one** sentence for `Dialogue` (which
really is missing, and is the `source` of both default ducks), not two. And a table whose duck names a bus that is
genuinely absent still says so, once.
