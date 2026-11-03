# research.md — iteration 72: case 31, and the decision nobody implemented

## S1 — the red case, and how long it had been red

`docs/loop/cues.yaml` case 31:

| | |
|---|---|
| behaviour | every number a cue author writes that the crate cannot honour is reported |
| observed by | `CueTable::problems()` over a cue with a `NaN` gain, a `NaN` slot gain, an inverted pitch range and a negative attenuation |
| green when | four sentences |
| **what it got** | **two** — `pitch` and `attenuation` reported; `gain_db` on the cue and on the slot **silent** |

The file's own `blocked_on` named `needs-decision/bad-gain-problem.md`, and that file had recommended **option C — a
sentence on `SirenLog`, no API change — since iteration 18**. So the case was not blocked. The recommendation was
written and never implemented, and each iteration read the file and moved on.

**The lesson is the loop's, not the crate's:** a decision file whose recommendation is actionable does not need the
owner's answer to proceed. It needs someone to read it and do the part it recommends.

## S2 — the technique: reuse the free-text field, add no variant

`Entry` is a public enum **without `#[non_exhaustive]`**, like `CueProblem`, so a new variant breaks every consumer's
exhaustive `match`. But iteration 16 had already solved this shape: `Entry::Stopped` carries a free-text
`why: String`, added for a one-shot layer that could not be described by a variant.

So the neutralisation goes there:

> `played, but cue gain_db and sound gain_db were not a number and each was read as the neutral value`

**No public enum gains a variant**, no consumer breaks, and the information is in the log a panel already draws. That is
option C exactly as the decision file scoped it.

## S3 — the plumbing, and why it is `Started`

`answer()` is a **pure function** — it takes the table, the settings and the request, and returns `Started` or
`Refusal`; it has no `ResMut<SirenLog>`. The neutralisation has to travel out of it, so `Started` carries
`neutralised: Vec<&'static str>`, in the same shape as its existing `stolen: Vec<(Entity, Cap)>`.

The names are the **field names a project's file uses** — `cue gain_db`, `sound gain_db`, `play gain_db`, `play pitch` —
so a person can search their table for the string in the message.

## S4 — what the behaviour is, and what it is not

**Unchanged:** a non-finite gain is still read as no gain, so the cue sounds at the level it actually authored. A lost
sound is worse than a mis-typed number, and that is what the decision file concluded.

**Changed:** silence. Before, a host inspecting `problems()` or `Refused` could not tell; now the log says so on the
frame the play is answered, beside the `Played`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `CueProblem` variants | 6, of which 2 cover `pitch` and `attenuation` | `src/cue.rs` |
| sentence count expected by case 31 | 4 | `cues.yaml` case 31 |
| sentence count delivered | **2 from `problems()` + 1 from `SirenLog`** | measured, `tests/neutralised.rs` |
| API breakage | **none** | no enum variant added, by construction |

## What good sounds like

A person who wrote a gain they computed, and occasionally got a `NaN`, needs one sentence that says **which number**,
**what was wrong**, and **what the crate did instead**:

> `played, but cue gain_db and sound gain_db were not a number and each was read as the neutral value`

Names both fields, says they were not numbers, and says the crate read them as neutral — so the reader knows the cue
sounded, knows why it sounded at that level, and knows what to fix. Singular and plural are both grammatical
(`the_sentence_is_grammatical_for_one_field_and_for_several`).
