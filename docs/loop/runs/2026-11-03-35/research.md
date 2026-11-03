# research.md — iteration 35: the report's doc listed four of its nine checks

## The item

`MusicScore::problems`'s doc:

> *"Everything wrong with the score, as sentences a person reads: a layer with no name, a layer naming a cue the table does
> not have, the same layer twice, a tempo or bar that is negative or not a number."*

**Four checks.** The function performs **nine**:

```
tempo_bpm is negative or not a number
beats_per_bar is not a positive number
the score has no layers
a music layer has no name
music layer `{}` is listed more than once
music layer `{}` has a gain that is not a number
… names cue `{}`, which the table does not have
… which is not a loop, so it cannot hold a layer
… which plays on bus `{}`; a layer is a stem of the `Music` bus
```

The doc was written when the report was new, and the two faults this loop added — a gain that is not a number (iteration 19)
and a layer on the wrong bus (iteration 34) — were never added to it. **A reader trusts a list and stops.** The same
understatement sits in `MusicLayer::cue`'s doc, which still says the cue *"should"* name the `Music` bus after iteration 34
made that **refuse** the layer.

## Sources

**S1. `MusicScore::problems`'s own doc**, and the function's nine `out.push` sites — counted, not remembered.

**S2. Iteration 34's rule** — a doc that says *"should"* where the code **does** is the inverse of the defect that iteration
found (*"should"* where the code did nothing). Both are the same failure: the doc and the mechanism disagreeing.

**S3. R6 (vendored: `docs/research/siren.md`)** — *"everything a play decides is **readable**"*. A report whose own description
omits half its checks is readable only to someone who reads the code.

**S4. The skill's step 4** — *"Fail loud, one path per feature."* The report **is** that path, and its doc is what an author
reads first.

## The technique

1. **`problems()`'s doc becomes the whole list** — nine items, each one sentence, with a note that the plugin **enforces**
   the last two and writes the same sentence to the log.
2. **`MusicLayer::cue`'s doc says `must`, not `should`**, and what happens when it does not.
3. **A test that walks the nine** — one score carrying every fault, asserting each sentence appears, and the empty-score case
   for the ninth. The doc and the code now have the same length, and a tenth check without a doc line fails the test only if
   the test names it — so the test is the **contract**, and the doc quotes it.

**A sweep, not a single fix.** The lead was *"grep every `should` in `src/`"*: five hits, two real (`MusicLayer::cue`'s
`should`, and the report's list). The rest are comments describing history, which are true as written.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the doc lists four checks | 4 | S1 |
| the function performs nine | 9 | S1 |
| the plugin enforces the last two, one line each | — | S2 |
| the report is the fail-loud path, and its doc is read first | — | S3, S4 |

## What good sounds like

A reader of `problems()`'s doc learns **all nine** things the report catches without opening the function, and a reader of
`MusicLayer::cue` learns that a cue on another bus is **refused**, not merely discouraged. A test walks the nine, so the
list cannot drift again in the direction it drifted twice.
