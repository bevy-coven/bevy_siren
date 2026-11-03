# review.md — iteration 72, three passes

## Pass 1 — Function: is case 31 green?

| `cues.yaml` case 31 | Before | After |
|---|---|---|
| `pitch` inverted → reported by `problems()` | yes | yes |
| `attenuation` negative → reported by `problems()` | yes | yes |
| **cue `gain_db: NaN` → named** | **no** | **yes**, in `SirenLog` |
| **slot `gain_db: NaN` → named** | **no** | **yes**, same sentence |
| the cue still plays at the level it authored | yes | yes |

| Test | Result |
|---|---|
| `a_cue_gain_that_is_not_a_number_is_reported_beside_its_play` | names the field, says what was wrong, says what it did, and the cue plays with a finite gain |
| `a_finite_gain_says_nothing_at_all` | **the report that must not fire** — no sentence for an ordinary gain |
| `the_sentence_is_grammatical_for_one_field_and_for_several` | singular for one field |
| `the_play_still_reports_the_gain_the_device_will_use` | a `NaN` sound + a −3 dB cue reports exactly −3.0 |

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Three defects while building it, all found by running:

| Defect | Found by | Fix |
|---|---|---|
| `neutralised` was declared **after** its first use (the pitch check) | the compiler | declaration moved above the pitch handling |
| the test app had **no `Play` message and no loaded sound**, so nothing played and the log stayed empty | four failing assertions showing `[]` | `helpers::app(...)` with `helpers::sounds(&["step_01"])`, then `write_message(Play::cue(...))` |
| `mod helpers;` in the test file — `helpers` is a **sibling of `all.rs`**, not of each test | the compiler | `crate::helpers::app(...)` |

The second is the informative one: a test that asserts on a log and never triggers a play **passes vacuously** if its
filter happens to match nothing. It failed here only because the assertion was `assert_eq!(said.len(), 1)`. A weaker
assertion — `assert!(said.iter().all(...))` — would have been green on an empty log and proved nothing.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: is it usable without me?

The sentence a panel shows:

> `2.00s stopped 'footstep.grass': played, but cue gain_db and sound gain_db were not a number and each was read as the
> neutral value`

It names **which** numbers, says **what** was wrong, says **what** the crate did instead, and starts with `played` so a
reader is not left thinking the cue failed. The field names are the ones a project's file uses, so the string can be
searched for.

- **No API break**: no enum gained a variant; a consumer matching `CueProblem` exhaustively is unaffected.
- **The guide now describes what happens** rather than what is missing, and still tells a developer to check
  `is_finite` before the table — the log tells you after, the check tells you before.
- **`cues.yaml` case 31 is green**, so the loop stops reporting it.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | case 31 — a silent neutralisation, red for ~30 iterations |
| tests | **+4**, from 352 to **356** all-features |
| suites | 198 default / 356 all-features |
| case | 31 red → **green** |
| API | **unchanged** — the free-text `why` on `Entry::Stopped` carries it |

## The process failure this iteration corrects

The case was red because the loop treated `needs-decision/` as a **blocker** rather than as **a file that can recommend**.
`bad-gain-problem.md` has said *"C now"* since iteration 18, and C needs no owner's answer. Thirty iterations read it and
moved on.

That is worth stating in the loop's own terms: **a decision file is not always a stop sign. When its recommendation is
actionable and non-breaking, the recommendation is the instruction.**
