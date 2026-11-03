# Iteration 106 — perf

Machine: this Mac, shared and loaded. Nothing here is on any hot path.

## Per-frame audio cost (control)

| voices | this run | band across unchanged code |
|---|---|---|
| 8 | 0.056 | 0.041–0.070 |
| 64 | 0.321 | 0.107–0.395 |
| 256 | 1.236 | 0.401–1.339 |

Inside the band iteration 102 recorded. The change adds a `const` array and a `len()` comparison **in a test**, and no
code at all on the mix path, so there is nothing to move.

## What the change costs at run time

**Zero.** `SETTINGS_CHECKS` is a `&'static [&'static str]` that exists only so the doc and the test can name the same set;
`problems()` does not read it. The alternative — having `problems()` iterate the constant — would have been a design where
the list *drives* the checks, which sounds tidier and is worse: the checks take different arguments and produce different
sentences, so a table-driven `problems()` would be a `match` on a name inside the same function.

## What it saves

A check that is silently dropped from the report, or added without a test, is a fault an author never sees. One
`assert_eq!` at test time is the cheapest possible insurance against a report that quietly stops reporting.
