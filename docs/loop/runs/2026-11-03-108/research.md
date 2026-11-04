# Iteration 108 — research: a guard that fired, and a variant that got through anyway

## The item

Iteration 107's recorded follow-up: *"`CueTable::problems` is the third of three with this shape and still has no
constant."*

## Measured

| | count |
|---|---|
| `CueProblem` variants | **7** |
| rows in `problems_names_each_mistake`'s expected list | **6** |
| fixture cues that produce a variant | **6** |
| the variant no fixture cue produced | `InertAvoidRepeat`, added in **iteration 105** |

The suite was **green** with seven variants and six rows. The hole is **narrower than "untested"**: `tests/play.rs` asserts
`InertAvoidRepeat` fires (and that legal windows and `Shuffle` do not trigger it) and `tests/model.rs` proves the behaviour.
What `problems_names_each_mistake` lacked is the *table-level* property — that every kind appears in a report and prints as a
sentence. It is the same pattern as iterations 106 and 107 — one decision (the set of problems) written in several places
with nothing forcing them to agree — found the same way, by counting.

| iteration | type | gap |
|---|---|---|
| 106 | `SirenSettings::problems` | doc said 12, code 13, test 11 |
| 107 | `MusicScore::problems` | counts agreed by hand; a tenth check passed silently |
| 108 | `CueTable::problems` | seven variants, six fixture rows |

## A claim of mine that was wrong, and how I found it

This file first said *"a guard that a variant is in that list: **none**"* and proposed an exhaustive `match`. I wrote the
`match`, added a tenth variant, got `error[E0004]`, and called that proof.

**Both halves were wrong, and `git diff` showed it.** The closure `named`, at `HEAD`, already listed all seven variants in
one or-pattern with no wildcard. So a compile-time guard **already existed** — and it had already **fired**:
`needs-decision/diagnostic-enums-are-extensible.md` records

```
error[E0004]: non-exhaustive patterns: `&CueProblem::InertAvoidRepeat { .. }` not covered
  --> tests/table.rs:66
```

in iteration 105. **That firing is how the gap happened.** The compiler demanded an arm; I wrote the arm; nothing demanded a
fixture. My experiment proved nothing, because I never ran it against the control — `HEAD` gives the same `E0004` for the
same tenth variant. It is the same mistake as iteration 107's mis-set experiment: an experiment that does not separate the
guard from what already exists.

Two further findings from reading the closure:

- Its comment said it *"reads the name rather than enumerating the variants"*. **The code enumerates every variant**; the
  comment was false.
- Its assertion, `problems.iter().any(|p| named(p, name))`, is **vacuous**: the line above it,
  `problems.contains(&problem)`, compares the whole value including the name, so `named` cannot fail on its own. It was a
  third copy of the variant list that checked nothing.

## Sources

| Source | What it reports |
|---|---|
| Ousterhout, *A Philosophy of Software Design*, Summary of Red Flags | **Information Leakage** — *"a design decision is reflected in multiple modules"*: here the enum, the expected list, the closure and the fixture |
| `needs-decision/diagnostic-enums-are-extensible.md` (iteration 105) | the measured firing of the exhaustive-`match` guard — evidence that it forces an arm and nothing more |
| iterations 106–107 (this crate) | the `const` repair for the two `Vec<String>` reports, whose return type has no machine-readable set |

## The technique

An exhaustive `match` forces **acknowledgement** of a new variant. A new variant needs **coverage**: some fixture cue that
produces it. The two come apart, as iteration 105 showed.

**One list, from which both the `match` and the count are generated.** A `macro_rules!` takes the variant names once and
expands to a `const KINDS` (the names) and a `fn kind(&CueProblem)` (an exhaustive `match`, no wildcard). Then:

- add a variant to the enum → **compile error** naming it, because the generated `match` lacks it;
- add it to the macro list, as the compiler demands → the list is one longer, so the test **fails** with
  `no fixture cue produces ["Tenth"]` until a fixture produces it.

The alternatives, and why not: `std::mem::variant_count` is unstable; `strum` is outside the closed dependency list; a scan
of `src/cue.rs` for `enum CueProblem` is a source-text test, which this project does not keep.

## Parameters

None.

## What good looks like

A maintainer adding an eighth `CueProblem` gets two failures in order: a compile error naming the variant, then — after the
one line the compiler asks for — a test failure naming it again, which only a fixture cue clears. **Neither can be
satisfied by writing an arm.**
