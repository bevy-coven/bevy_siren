# plan.md — iteration 45: the mix table's report is held to the standard its sibling meets

## The pick, and why it beats the runner-up

**Pick:** `SirenSettings::problems()` emits **nine sentences**; `MusicScore::problems()` emits nine and has
`a_score_reports_every_fault_its_doc_lists` walking all of them. Measured by fixed-string search over `tests/`, **four
of the mix table's nine had no test at all** — `master_db is not a number`, `a bus has no name`, `a bus … has a gain
that is not a number`, and `a duck … has a hold or fade that is negative`. Its doc named **six** of the nine.

**Making that matter found the bigger one:** the panel's `Problems` block read **only** `table.problems_in(&settings)`
— the *cue* table's faults — so a mix table with `max_voices: 0`, a NaN master or a bus listed twice drew
`Problems: none` **while every bus read −80 dB**. The report's other half had never been on screen.

Beats the runner-up (the `Music` header's live-layer count): that is decoration on a block that already names each
layer; this is a report that says `none` about a broken table.

## The blocked item, re-checked

**`cues.yaml` case 31** (a non-finite cue `gain_db` reported by `problems()`) — still blocked on
`needs-decision/bad-gain-problem.md`; every route touches a public enum with no `#[non_exhaustive]`, and its own
recommendation defers the variant to the next minor (the crate is 0.1.0). Nothing built from it.

## The design

1. **`SirenSettings::problems`'s doc lists all nine checks**, as `MusicScore::problems`'s does since iteration 34.
2. **`tests/mix.rs::problems_names_every_mistake`** — one table holding every fault, each sentence asserted, each
   shown to be **its own** fault (a table with only the duplicate bus reports only the duplicate), the `NaN` half of
   the hold/fade sentence covered separately, and a good table asserting nothing.
3. **The panel's `Problems` block draws both reports**: `settings.problems()` first, then `table.problems_in(&settings)`.
4. **`SIREN_MIX=broken`** gives the *mix* table faults (a NaN master, a duck whose hold is negative) so the block can be
   seen drawing them.
5. **The cue list is bounded at `CUES_SHOWN`** with `and N more`, because `SIREN_FAULTS=many`'s twelve rows pushed the
   log off the bottom entirely — measured, the log's heading was not on screen.
6. **The log's line count follows every row above it**: the `Problems` block's lines *and* the cue list's `and N more`
   row.

## Fields

None. No new field on `CueTable`, `SirenSettings` or `MusicScore`; no public type changed.

## Modules touched

- `src/mix.rs` — the doc's list (six → nine). No behaviour change.
- `tests/mix.rs` — `problems_names_every_mistake`.
- `examples/mixer.rs` — the block's two reports, `SIREN_MIX=broken`, the cue-list bound, the log's count.

## Tests

- **`problems_names_every_mistake`** — nine sentences, each separable, plus the `NaN` hold and a clean table.
- **Mutation-checked per sentence:** removing any of the four previously-unreached `out.push` lines reddens it
  (verified one at a time: `master_db`, `a bus has no name`, the bus gain, the hold/fade).

## The carve

- **Not** a `CueProblem` variant for a non-finite gain — that is case 31 and its decision file.
- **Not** a separate `Mix problems` heading: the panel's column is fixed and has no scroll, and one `Problems` block
  answering *"is this table fit to run"* is one question, whichever file the fault is in.
