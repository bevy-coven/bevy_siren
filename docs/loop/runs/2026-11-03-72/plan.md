# plan.md — iteration 72: case 31, the cheapest large win

## The pick, and why it beats the runner-up

**Pick:** `cues.yaml` case 31 — a `NaN` gain that the crate neutralises in silence.

**Runner-up:** nothing on iteration 71's list is decidable. Three of the five items are decisions (the weights schema,
the labelled set, the licence audit) and the fourth needs the encoder those decisions gate.

**One sentence:** case 31 is the only red case in the crate, it has been red for thirty iterations, and the decision file
it was blocked on **already contains the recommendation that fixes it** — so it is not blocked at all, it is unimplemented.

## The change

| Item | What it is |
|---|---|
| `Started::neutralised` | which fields of a play the crate had to rewrite, by the name a project's file uses |
| the log line | `Entry::Stopped { why }` carries the sentence, so **no public enum gains a variant** |
| `cues.yaml` case 31 | red → **green**, with the new observation |
| `docs/guide/cues.md` | the "this is missing" note becomes "this is how it is reported" |

## Why the log and not `CueProblem`

`CueProblem` is a public enum **without `#[non_exhaustive]`**, so a seventh variant breaks every consumer that matches it
exhaustively — a hard break in a 0.1.x patch. `needs-decision/bad-gain-problem.md` weighed four options and recommended
**C now, A at the next minor**. C is what shipped: `Entry::Stopped`'s free-text `why`, a shape iteration 16 already
established for exactly this reason.

## Every number, and its source

No new constants, no new fields on a serde type: the change adds a `Vec<&'static str>` to a private struct and one log
line. The behaviour — a non-finite gain reads as no gain — is unchanged from iteration 18, and `finite_or` is untouched.

## The tests that prove it

| Behaviour | Test |
|---|---|
| a `NaN` cue gain is reported beside its play | `a_cue_gain_that_is_not_a_number_is_reported_beside_its_play` |
| the cue still **plays** (neutralising, not refusing) | same test — one `Played`, `gain_db` finite |
| a finite gain says nothing | `a_finite_gain_says_nothing_at_all` |
| one field and several read as English | `the_sentence_is_grammatical_for_one_field_and_for_several` |
| the meter and the device agree | `the_play_still_reports_the_gain_the_device_will_use` |

## What was carved off

**Option A** — the `CueProblem` variant — is still the tidier home and still costs a breaking change. It waits for a
minor release, exactly as the decision file says. The `needs-decision/` file is **not** updated to "decided" by this
iteration: option A remains open, and what shipped is the part of C that was already recommended.
