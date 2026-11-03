# plan.md — iteration 52: a duck that lowers nothing says so

## The pick, and why it beats the runner-up

**Pick:** `SirenSettings::problems()` reports a duck whose `by_db` is exactly `0.0` — a rule that fires, is logged, holds
its `hold_s`, and lowers the bus by nothing.

Beats the runner-up (an `AvoidRepeat` window that cannot be honoured): that needs a **seventh `CueProblem`**, gated by
the open `bad-gain-problem.md` enum decision; this is **additive** — `SirenSettings::problems()` returns `Vec<String>`,
so a sentence breaks no consumer, and it is the same shape as iteration 48's bus-cap sentence.

## The ranked list

1. **BUG — a duck at `by_db: 0.0` is inert and unreported.** Shipped this iteration.
2. `CueProblem` for an `avoid` window that cannot be honoured — **blocked** on the enum decision.
3. `cues.yaml` case 31 (a non-finite cue `gain_db` is neutralised silently) — **blocked** on the same decision.
4. `reset_histories` has no caller — a consumer item for `bevy_grimoire`, its own commit.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; the guide says what to turn, a lower default is a decision.

## The design

**One sentence in `SirenSettings::problems()`**, beside the existing no-hold/no-fade sentence:

```
the duck on `Music` lowers it by 0 dB, which is nothing at all; set by_db to the reduction you want
```

placed **before** the no-hold/no-fade check, because a `0.0` with a hold and a fade is a different fault from a `0.0`
without one, and an author who set both should be told the depth is the problem, not the envelope.

**Why not report `NaN` too:** RON refuses `nan` at parse (`measured: "Expected float"`), so it is not authorable; a
non-finite `by_db` built in code is already neutralised by `active_duck_db`'s own guard, which is the crate's documented
rule for a non-finite number.

## Fields

**No new field, no new API, no schema change.** The rule reads the existing `Duck::by_db`.

## Tests

- **`tests/mix.rs::a_duck_of_zero_db_says_it_lowers_nothing`** — `SirenSettings { ducks: vec![Duck::new(..).by_db(0.0)
  .hold_s(0.25).fade_s(0.6)] }.problems()` is exactly one sentence naming the bus and the `0`.
- **`tests/mix.rs`** — the crate's own `default_ducks()` (Gregory's −8 dB) reports **nothing**: the report must not fire
  on a real duck. This is the "refusal that must not happen" case.
- **`tests/mix.rs`** — `by_db: -0.0` is not reported twice and `by_db: -8.0` with a zero hold still gets only the
  no-hold sentence, so one fault stays one sentence.
- A play with a `0.0`-dB rule still **plays and ducks nothing** — the behaviour is unchanged; only the report is new.
