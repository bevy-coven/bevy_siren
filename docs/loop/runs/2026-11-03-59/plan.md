# plan.md — iteration 59: an empty bus is not a bus mixed to the floor

## The pick, and why it beats the runner-up

**Pick:** `worst_peak_db` returns `SILENT_DB` for a bus with **no voices**, which is the same number it returns for a bus
**mixed to the floor** — while its own doc says *"nothing playing is not `SILENT_DB` — it is no peak at all"*. The
crate's own example filtered `voices_on > 0` first, which is the workaround a collision forces on every caller.

Beats the runner-up (a `Refusal` for a wrong-bus stinger): that needs the public-enum break on a **published** crate.

## The ranked list

1. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
2. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
3. **`worst_peak_db`'s sentinel collides with a real floor reading.** Shipped this iteration.
4. A stinger on the wrong bus writes only a log line, not a `Refused` — `needs-decision/stinger-refusal.md`, same
   decision.
5. `load_table` does not call `retain_keys` itself — changes a documented contract; wants a decision file.

## A finding that was **not** a bug, and is now a decision file

This iteration probed the hypothesis that a refused play should not advance the variant window. It **does**, and that is
**protective**: measured, a cue with one missing file still sounds **4 of 8** requests. Without advancing it would
re-pick the missing sound for ever. Recorded in `needs-decision/pick-before-validate.md` with the measurement, so a later
iteration does not "fix" it.

## The design

- **`SirenMix::peak_db(bus) -> Option<f32>`** — `None` for no voice, `Some(db)` for a real bound. **Additive.**
- **`worst_peak_db` keeps its sentinel**: one number for every bus, and changing its signature would break every
  consumer of a published crate.
- **The example uses the new method**, deleting its `n > 0` filter — the caller that found the collision is the first to
  stop working around it.

## Fields

**No new field, no schema change.** One additive public method.

## Tests

- **`tests/mix.rs::an_empty_bus_is_not_the_same_peak_as_a_silent_one`** — `peak_db` is `None` with no voices,
  `Some(SILENT_DB)` for a bus mixed to the floor; `worst_peak_db` still collides, and the test says so rather than
  pretending the sentinel moved.
- **`tests/mix.rs`** — a bus at −70 dB with two voices is **above** the floor, so it never collided.
- The example's panel still renders, and no peak clause appears — captured.
