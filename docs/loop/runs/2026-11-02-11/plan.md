# plan.md — iteration 11: pin the half of "never stolen" that had no test

## The pick, and why it beats the runner-up

**Pick:** `UNSTEALABLE_PRIORITY` means *never taken*, whatever arrives. The suite proves it, but only through a path a
reader has to reason about — the table-budget test's second request is the *same* unstealable cue. Adding the direct
case (a priority-0 request against a priority-200 voice) makes the rule legible. **My first reading — that the promise
had an untested half — was wrong, and the mutation check in Verification proved it**; the plan is corrected here rather
than in the log alone.

**Runner-up:** `DuckClock.started`'s staleness (iteration 10's next-5 head) is **not a bug** — measured: 50 fires over
two rules leave two entries, and `duck_db` reads only pairs still in `settings.ducks`. Closed by measurement, not by a
change.

The pick wins because it is the only remaining item that is (a) unblocked and (b) about the crate's strongest promise.
It closes **no hole** — the mutation shows the old test catches the same break — and that is stated rather than sold as
a repair. Ten iterations have closed every rank-1 defect the checks could find; this one makes the most important rule
read the way it is written.

**Carved off:** nothing. The change is tests plus a `cues.yaml` note; no `src/` file moves.

## Design

`tests/mix.rs`, two tests, deliberately paired:

1. `an_unstealable_voice_is_never_taken_by_a_stealable_request` — a table budget of 1, one unstealable voice live, a
   **priority-0** request: `Refused { VoiceLimit { bus } }`, the live count stays 1, the unstealable voice keeps its
   entity, and `SirenLog` holds **no** `Stolen`/`stolen by` line for it. Fails if `model::steal` ignores `unstealable`.
2. `a_stealable_voice_plays_beside_an_unstealable_one` — the pair that stops the first from passing vacuously: with the
   table at 2 and one unstealable voice live, a stealable request **plays** and the unstealable one is untouched. A build
   that refused too much is red here — the same rule `cues.yaml`'s note states for cases 10–13.

`docs/loop/cues.yaml` case 9 gains the observation it always meant: *"and a stealable request cannot take one"*.

## Every field touched on a serde type

None. No `src/` file changes.

## Verification

- the whole step-1 baseline and the `cues.yaml` table (case 9 and its neighbours must stay green);
- **the mutation, which corrected this plan**: remove `!c.unstealable` from `model::steal`'s filter and observe **three**
  tests go red — the two new ones *and* `the_table_budget_refuses_rather_than_grow`. The suite already covered the
  promise; the new tests make it direct. The measurement is in `review.md`;
- `cargo run --release --example perfcheck` before/after: no `src/` change, so the numbers are the machine's and the
  report says so;
- `examples/render` before/after: cue WAVs byte-identical, since nothing a play decides moves.

## Risk

A test-only iteration risks being a non-deliverable — a pass that ships nothing. This one ships a **guard** on the crate's
strongest claim plus the measurement that retires an item from the backlog. It is smaller than the last ten iterations
and is reported as such: no defect was found, and the honest outcome is that the rank-1 surface is clean and the rest is
decision-blocked. The log says that plainly rather than manufacturing a fix.
