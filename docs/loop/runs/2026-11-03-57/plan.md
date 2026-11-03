# plan.md — iteration 57: a stinger that cannot dip does not claim it did

## The pick, and why it beats the runner-up

**Pick:** `arm_stinger_ducks` armed a duck and logged `Ducked { by_db: 0.0 }` for a stinger whose depth cannot lower
anything — `active_duck_db` returns `0.0` for any `by_db >= 0.0`. Measured: the log said `[0.0]` and `Music` did not
move.

Beats the runner-up (`AvoidRepeat`'s unreported window) because that is **blocked** on the open `CueProblem` enum
decision, and this is a **wrong log line** — rank 1 — needing no API change at all.

## The ranked list

1. **A stinger at `duck_db: 0.0` (or positive) logs a duck it did not make.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.
5. `reset_histories` has no caller — a consumer item for `bevy_grimoire`.

## The design

**One condition in `arm_stinger_ducks`**: arm and log only when `duck_db.is_finite() && duck_db < 0.0`. That is the same
test `active_duck_db` uses to decide whether the number means anything, so the log and the envelope agree by
construction.

**The stinger's own hit still plays.** Only the duck is conditional: a zero-depth stinger is a sound with no dip, which is
a legitimate authoring choice, and silence about it is correct — nothing lied.

**Not built: a `Refusal` for it.** A stinger is a message a game sends at runtime; refusing it would need a new
`Refusal` variant and would stop a sound a player should hear. The crate's rule is *"a bad cue is refused, never played
wrong"* — a stinger with an inert depth is not played wrong, it is played with no duck.

## Fields

**No new field, no API change.** One condition.

## Tests

- **`tests/play.rs::a_stinger_at_zero_db_logs_no_duck`** — `duck_db: 0.0` writes **no** `Ducked` line. **Fails on the old
  tree** with `a 0 dB stinger must not claim a duck: [0.0]`.
- **`tests/play.rs::a_negative_stinger_duck_still_logs_and_dips`** — the non-regression: −9.0 logs `[-9.0]` and `Music`
  reads below −0.5 dB.
- **`tests/play.rs::a_raising_stinger_does_not_claim_a_duck`** — `+6.0` writes no `Ducked` line either.
