# Iteration 107 — research: the same list, in the other score

## The item

Iteration 106's recorded criticism: *"`MusicScore::problems` claims the same shape — nine checks — and is one inspection
from the same drift."*

**The criticism was half right, and the correction matters.** A test *does* exist
(`a_score_reports_every_fault_its_doc_lists`, 9 needles, with its own doc recording that the list once said four while the
code performed nine). What does **not** exist is anything enforcing the *other* half of the doc's claim.

## Measured

| | count |
|---|---|
| checks the code performs | **9** |
| the doc's prose claims | **9** ✓ |
| the doc's bullet list describes | **7** (two bullets name two faults each) |
| needles in the test | **9** ✓ |
| a **tenth** check added to `problems` with no matching name | **passes silently** — measured by adding `if self.tempo_bpm > 1000.0 { … }` |

So the counts agree *today*. What is missing is the guarantee:

> **Everything wrong with the score**, as sentences a person reads. Nine checks, and **this list is the whole of them** — a
> doc that says "should" or that lists four of nine leaves a reader believing the crate catches less than it does

The test proves **"each documented fault fires"**. It never proves **"the set is exactly these"**: the nine are correct
**by hand**, checked only when someone counts. A tenth check was added to the function, the suite stayed green, and the doc
would have gone on saying *"this list is the whole of them"* while it was not.

This is the same defect as iteration 106's, one degree milder: the count was right by luck rather than by construction.

## A measurement I got wrong, and the correction

My first attempt to prove the fix added the tenth **check** to the function without adding a tenth **name** to
`MUSIC_CHECKS` — and the test still passed, which looked like the guard failing. It was not: the constant is the list, and
the assertion compares needles against **the constant**, so ten code checks and nine names is a state the constant does not
describe. Adding both (the check *and* its name) fails at **`9 != 10`**, naming the list.

That distinction is the design, not a loophole: a check that is not in `MUSIC_CHECKS` is not a documented check, and the
constant is the one place a maintainer declares one. The first measurement was wrong, and saying so is cheaper than a
reader discovering it later.

## Sources

| Source | What it reports |
|---|---|
| Ousterhout, *A Philosophy of Software Design*, Summary of Red Flags | **Information Leakage** — the set of checks is one decision, written in the doc's prose, the doc's bullets and the test's needles, with nothing forcing agreement |
| iteration 106's `mix::SETTINGS_CHECKS` (this crate) | the repair pattern, already proved: one `const` the doc names and the test walks. `problems_names_every_mistake` for the mix table now fails when a check has no needle |
| the test's own doc (this crate) | *"the doc was written when the report was new, and the two faults this loop added … were never added to it"* — the drift has happened here once already, in the **other** direction |

## The technique

**`MUSIC_CHECKS: &[&str]`, the one list.** The doc points at it instead of counting. The test walks it, so a tenth check
without a needle fails and names the list — the exact guard iteration 106 proved fails when it should.

## Parameters

None. No rate, curve or threshold changes.

## What good sounds like

A maintainer adding a tenth check to `MusicScore::problems` gets a failing test that prints the list and says which name is
missing. The doc cannot go stale, because it no longer holds a number.
