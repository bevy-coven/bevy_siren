# plan.md — iteration 58: a layer change that names nothing says so

## The pick, and why it beats the runner-up

**Pick:** `move_layers` dropped a `MusicLayers` target whose layer the score does not have, with **nothing said** — the
one message in the crate with no failure path. Measured: a typo produced **zero** log lines.

Beats the runner-up (a `Refusal::StingerOffMusic` variant): that needs the public-enum break three open decisions wait
on; this needs **no API change at all**, and it is the same class — an accepted message silently discarded — as the
refusals the crate already ships.

## The ranked list

1. **A `MusicLayers` target naming a missing layer is silently dropped.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. A stinger on the wrong bus writes only a log line, not a `Refused` — `needs-decision/stinger-refusal.md`, same
   decision.
5. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.

## The design

- **`let … else` instead of `if let`** in `move_layers`, writing `Entry::Refused { cue: name, reason }` and `continue`.
  One `find` decides both branches, so the miss costs no extra lookup.
- **`SirenLog`, not a new `Refusal` variant.** The score is not the mix table, so `UnknownBus` would be false; a new
  variant is the break three decisions already wait on. Iteration 9 set this precedent for the stinger's wrong bus.
- **`SIREN_LAYER=typo`** in `examples/mixer`, so the sentence is capturable and a reader can see it without the code.

## Fields

**No new field, no API change.** One control-flow shape and one log line.

## Tests

- **`tests/play.rs::a_layer_change_naming_a_missing_layer_says_so`** — the log names `nope`. **Fails on the old tree**
  (measured: the log held only the layers' own `played` lines).
- **`tests/play.rs::a_real_layer_change_moves_and_says_nothing`** — the non-regression: a real layer's target moves and
  no refusal names it.
- **`tests/play.rs::a_typo_says_which_layer_and_a_real_one_still_moves`** — **one change with both**: `combat` moves to
  −3.0 dB and `cmobat` is named, so the fix cannot pass by refusing wholesale.
