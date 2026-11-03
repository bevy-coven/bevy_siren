# research.md — iteration 45: the mix table's report is held to the standard its sibling already meets

## The item

`SirenSettings::problems()` emits **nine distinct sentences**. `MusicScore::problems()` emits nine too, and has a test
(`tests/music.rs::a_score_reports_every_fault_its_doc_lists`) that builds one score containing every fault, asserts each
sentence appears, and closes with a comment naming the ninth that needs a score of its own. **`SirenSettings::problems()`
has no such test.** Measured, by fixed-string search over `tests/`:

| sentence | reached by a test? |
|---|---|
| `max_voices is 0, so no sound can ever play` | yes |
| `master_db is not a number` | **no** |
| `a bus has no name` | **no** |
| `bus `<n>` is listed more than once` | yes |
| `bus `<n>` has a gain that is not a number` | **no** |
| `a duck starts on bus `<b>`, which the table does not have` | yes |
| `a duck lowers bus `<b>`, which the table does not have` | yes |
| `the duck on `<b>` raises it by `<n>` dB; a duck lowers, so by_db should be negative` | yes |
| `the duck on `<b>` has a hold or fade that is negative or not a number` | **no** |

**Four of nine unreachable.** A report sentence nothing can trigger is a sentence nobody has read aloud, and a report is
the one thing a panicking audio system cannot afford to get wrong — it is what an author trusts instead of the sound.

## Sources

**S1. `MusicScore::problems`'s own test** — `tests/music.rs::a_score_reports_every_fault_its_doc_lists`, whose comment is
*"`MusicScore::problems`'s doc lists all nine checks it performs … and a test walks all nine so the list and the code
cannot drift apart again."* Written in iteration 34 after the score's doc named four checks and the code performed
nine. **The same drift was never checked on the mix table.**

**S2. `SirenSettings::problems`'s doc** — *"Everything wrong with the table, as sentences a person reads on an editor's
status line: a bus listed twice or unnamed, a budget of zero, a duck that names a bus the table does not have, a duck
that raises rather than lowers."* It names **six** of the nine. `master_db is not a number`, a bus's gain that is not a
number, and a duck's hold or fade are unlisted — the exact defect S1 fixed in the score.

**S3. R6, vendored at `docs/research/siren.md`** — *"a play is never silent: `Played` or `Refused`, both with the reason
… everything a play decides is readable."* A report is the readable half of that rule; a sentence no test reaches is a
claim about readability nobody has checked.

**S4. `tests/mix.rs::problems_names_each_mistake`** — the existing test. It builds one table with a duplicate bus and
three duck faults, and asserts four sentences plus `SirenSettings::default().problems().is_empty()`. It is a good test;
it stops at four.

## The technique

One test, shaped like S1's: build **one** table containing every fault the report knows, assert each sentence appears,
then assert each fault is separable — every sentence disappears when its own fault is fixed, so no sentence can be
emitted by something else. And fix the doc so its list matches the code (S2).

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| sentences the score's report emits, all tested | 9 | S1 |
| sentences the mix table's report emits | 9 | `src/mix.rs` |
| of those, reached by a test today | **5** | measured above |
| sentences the doc names | 6 | S2 |
| a report that names a fault no test reaches | a claim nobody checked | S3 |
| the existing test's scope | 4 sentences | S4 |

## What good sounds like

A test whose comment reads like S1's — one table, nine sentences, and the ninth-with-no-other-home called out by
name — and a doc whose list has nine items rather than six. A reader who trusts the report can see that every line it
can print has been made to print at least once.
