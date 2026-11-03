# Iteration 107 — perf

Machine: this Mac, shared and loaded. Nothing here is on any hot path.

## Per-frame audio cost (control)

| voices | this run | band across unchanged code |
|---|---|---|
| 8 | 0.061 | 0.041–0.070 |
| 64 | 0.353 | 0.107–0.395 |
| 256 | 1.335 | 0.401–1.339 |

Inside the band iteration 102 recorded — the 256-voice figure is at its top edge and the 64-voice figure is mid-range, both
of which that iteration documented as the machine's spread rather than a result. The change adds a `const` and a test
comparison, and **no code at all on the mix or music path**.

## What the change costs at run time

**Zero.** `MUSIC_CHECKS` is a `&'static [&'static str]` that `problems()` never reads; it exists so the doc and the test can
name one set. The alternative — driving `problems()` from the constant — would put a `match` on a name inside the function
that already performs the checks, which is more indirection for the same guarantee.

## What it saves

A check that is silently undocumented, or a doc that claims completeness while the function grows. One `assert_eq!` at
test time against the cheapest possible class of bug: a report that stops reporting and says it has not.
