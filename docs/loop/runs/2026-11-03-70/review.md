# review.md — iteration 70, three passes

## Pass 1 — Function: does the type do what the issue asks?

| Acceptance criterion | Test | Result |
|---|---|---|
| round-trips through `from_ron` / `to_ron` behind `serde` | `the_weights_round_trip_through_ron` | field for field |
| all-zero refused with a message naming what is wrong | `all_zero_weights_are_refused_with_a_message_that_says_what_to_do` | names the four fields to set |
| every field's default documented with its source or **design default** | read in the source | all six say **design default** with the argument |
| a default produces a non-empty result on the fixture library | *not this iteration* — it needs `#40`'s ensemble | **carved off, stated in `plan.md`** |

The fourth is the one I did not meet, and it is recorded rather than quietly dropped: a default that produces a
non-empty result needs something to produce a result, which is `#40`, which needs an encoder behind a licence decision.

**Verdict: GREAT** on what it claims; the unmet criterion is named, not hidden.

## Pass 2 — Zoom: read the awkward cases

Two things this iteration deliberately gets right, and one it caught in itself:

| Case | What the code does |
|---|---|
| a **stylised** palette (`acoustic: 0.0`, `loudness_tolerance_db: 40.0`) | **allowed** — `an_unusual_but_possible_configuration_is_allowed_through` |
| a **NaN** weight | refused with `NotFinite`, because it compares `false` against everything and every comparison silently fails |
| **two** mistakes in one file | both reported — `every_problem_is_reported_not_just_the_first` |
| an **old file** with fewer fields | loads, gaining defaults — asserted, because it is a promise rather than an accident |
| a **typo'd field** | refused by `deny_unknown_fields`, so a setting that never took effect is not silent |
| a file that **parses** but means nothing | parses, then `problems()` refuses — the two layers do different jobs |

The boundary between *impossible* and *unusual* is the design decision here, and the tests cover both sides: five
refusals and three allowed choices. **A report that refuses a legitimate choice is a report people learn to ignore.**

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **The messages name the field, the value, why, and the fix**:
  > *"`acoustic` is -0.5: a negative weight makes a signal that should attract repel. Remove the minus sign, or set it to
  > 0 to ignore the signal"*

  Same shape as `Refusal` and `CueProblem`, which is the point of following the crate's pattern rather than inventing a
  fourth.
- **`shares()` answers "what was this result made of"** without the caller dividing four floats and hoping the total is
  not zero — and returns `None` rather than four NaNs when it is.
- **The default build is untouched**: 181 tests with no features; the module is behind `semantic`, and `serde` gates only
  the two I/O functions.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | `#41`, the ensemble weights as a serde type |
| new module | `src/semantic/weights.rs` |
| tests | **+15**, from 324 to **339** all-features |
| suites | 181 default / 190 serde / 191 mixer / **339 all-features** |
| carved off | nesting into `SirenSettings` → `needs-decision/`; the "non-empty result" criterion needs `#40` |

## A disk failure, recorded for the fourth time

Three suites failed mid-iteration with errors that **read as code defects**: `rustc-LLVM ERROR: IO failure on output
stream`, and `failed to write lib.rmeta`. Both were the disk at **130 MB**. A clean freed 18.6 GiB and every count above
was then taken from a complete build.

The pattern is now consistent enough to state as a machine fact: **a debug build of this crate plus the full `bevy`
dev-dependency is ~19 GB, and running more than two feature combinations in one iteration needs a clean between them.**
