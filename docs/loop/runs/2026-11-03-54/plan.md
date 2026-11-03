# plan.md — iteration 54: a duck rule the table dropped stops holding its start

## The pick, and why it beats the runner-up

**Pick:** `DuckClock::started` is never pruned — 2000 fires plus a `prune` at t=1000 still held **2000** entries, and 100
table edits left 100. It is an unbounded map of two `String`s per entry in a resource that lives for the process, and
the Cauldron edits that table live.

Beats the runner-up (`AvoidRepeat`'s unreported window): that is gated on the open `CueProblem` enum decision; this needs
no API break — `retain_rules` and `rule_starts` are **additive**, and `prune`'s signature is untouched.

## The ranked list

1. **BUG — `DuckClock::started` is never pruned.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. `reset_histories` has no caller — a consumer item for `bevy_grimoire`, its own commit.
5. `SirenSettings::master_db` defaults to `0.0` — no trim; a lower default is a decision file.

## The design

- **`retain_rules(&settings)`** — drops a `started` entry when no rule in `settings` names its `(source, target)`. Called
  by `update_mix` beside `prune`, so it runs every frame without a new public call a project must remember.
- **`rule_starts() -> usize`** — the retained size, so the fix is observable by a panel and assertable by a test rather
  than inferred from `Debug`.
- **`prune` is unchanged** in signature and behaviour: it drops finished *manual* envelopes, which is a question the
  clock can answer alone. Splitting the two questions is the whole fix.

## Fields

**No new field, no schema change.** Two additive public methods and one call in `update_mix`.

## Tests

- **`tests/mix.rs::a_rule_the_table_dropped_stops_holding_a_start`** — 2 fires, retain against a table naming one, 1
  start held; and 100 fires against a table naming one, 1 start held.
- **`tests/mix.rs::a_live_rule_keeps_its_start_across_a_retain`** — the non-regression: a running duck reads the same
  −8.0 dB at the same moment before and after a retain. A prune must not restart or shorten an envelope.
- **`tests/mix.rs::the_mix_prunes_a_table_it_no_longer_has`** — through the app: play, clear the table's ducks, one frame,
  `rule_starts() == 0`. The path a project actually runs.
- The measured "2000 entries after a prune" is recorded in `eval.md` as the before-number, since the new API cannot
  compile against the old tree to fail there in a test.
