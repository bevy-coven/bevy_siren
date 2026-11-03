# Iteration 106 — research: a guarantee the test did not provide

## The item

Not an open issue: a defect in the **documentation of `SirenSettings::problems()`**, found by counting.

Its doc said:

> **Twelve checks**, and `tests/mix.rs::problems_names_every_mistake` walks all twelve so this list and the code cannot
> drift apart

Measured:

| | count |
|---|---|
| checks the doc lists | **12** |
| checks the doc says the test walks | 12 |
| needles the test actually asserts | **11** |
| distinct checks the code performs | **13** |

The check the doc does not list, no needle covers, and no test asserts is:

```
the duck on `{}` lowers it by 0 dB, which is nothing at all; set by_db to the reduction you want
```

— a check whose own comment says it fixed a **measured** bug (*"Measured: `SirenSettings { ducks:
[Duck::new("Dialogue","Music").by_db(0.0).hold_s(0.25).fade_s(0.6)] }.problems()` was `[]`"*), and whose test coverage
exists in three separate places in `tests/mix.rs` but **not** in `problems_names_every_mistake`.

So the doc's central claim — *"this list and the code cannot drift apart"* — was false in the direction that matters. A
maintainer reading it would add a check, add a sentence to the list, and believe the test would catch a mismatch. It
would not.

## The source

| Source | What it reports |
|---|---|
| Ousterhout, *A Philosophy of Software Design*, **Summary of Red Flags** | **Information Leakage** — *"a design decision is reflected in multiple modules"*. The count of checks is one decision, and it was written in **three** places (the doc's prose, the doc's list, the test's needles) with nothing forcing them to agree |
| the same, on comments | a comment that restates the code is a second copy of the code and will drift from it; a comment that states an **invariant a test enforces** is a claim, and a claim without an enforcer is a liability |
| `MusicScore::problems`'s own doc (this crate) | the same pattern claimed and, by inspection, the same risk: *"Nine checks, and this list is the whole of it"* |
| Sun et al., `10.48550/arXiv.2406.19228` | **failure detection** — a report that cannot detect its own incompleteness is the tool-level version of the same fault |

## The technique

**Stop counting in prose, and make the set checkable.** Two changes, in this order:

1. **`SETTINGS_CHECKS: &[&str]`** — one `const` listing the checks by name, beside the function that performs them. The
   doc points at it rather than restating it, so **the prose cannot disagree with the list**: there is one list.
2. **The test walks `SETTINGS_CHECKS`** and fails when the list and the needles differ in size, naming the missing check.

That converts the doc's promise from **prose** into a **compile-time-adjacent invariant**: add a check without adding it
to the constant and the test says which one is missing.

**Why a constant and not a table of needles in the test.** The needles must stay in the test — they are assertions about
sentences, and a sentence is the thing being tested. The constant is the **index** that makes a missing needle visible
rather than merely absent.

## Parameters

None. This changes no rate, curve or threshold.

## What good sounds like

A maintainer adding a fourteenth check gets a failing test that names the check as missing from the list. The doc says
*"the list is `SETTINGS_CHECKS`"* and cannot be wrong, because it is not a second copy.
