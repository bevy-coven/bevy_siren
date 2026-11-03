# plan.md — iteration 55: a cue the table dropped stops holding a picker

## The pick, and why it beats the runner-up

**Pick:** `SirenChoices::pickers` is keyed by cue name, created on a cue's first play, and removed by nothing. Measured:
200 names played, table cut to 10, **200 pickers still held** — and each holds an owned copy of its recent sound keys.

Beats the runner-up (`AvoidRepeat`'s unreported window): that is gated on the open `CueProblem` enum decision; this needs
no API break — `retain_cues` and `picker_count` are **additive**.

## The ranked list

1. **BUG — `SirenChoices::pickers` is never pruned.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. `reset_histories` has no caller — a consumer item for `bevy_grimoire`, its own commit. (This iteration made the
   distinction sharper: `reset_histories` clears histories and is *not* a retain, so the panel's reload wants
   `retain_cues`, which the crate now calls itself.)
5. `SirenSettings::master_db` defaults to `0.0` — no trim; a lower default is a decision file.

## The design

- **`retain_cues(&table)`** — drops a picker whose cue the table no longer names. **One pass over the table** into a
  `HashSet`, not `pickers.retain(|n, _| table.get(n).is_some())`: `CueTable::get` is a linear scan, so the obvious form
  is O(pickers × cues) and measured **990 µs/frame** at 1000 cues. The one-pass form is **8.5 µs**.
- **`picker_count()`** — the retained size, so the fix is observable by a panel and assertable by a test.
- **Called from `play_requests` only when `table.is_changed()`.** A static table makes the same answer every frame;
  paying 8.5 µs per frame for a session-long constant is the same fault as paying 990.
- **An early return when the map is empty**, which is every frame before the first play.

## Fields

**No new field, no schema change.** Two additive public methods and one guarded call.

## Tests

- **`tests/play.rs::a_cue_the_table_dropped_stops_holding_a_picker`** — 200 names played, table cut to 10, ten pickers.
- **`tests/play.rs::the_picker_map_grows_per_cue_name`** — asserts the 200, that `reset_histories` leaves the map at
  200 (so the reader sees why it was not the fix), and that the cut table reaches 10. Uses `Debug`, so it **runs on the
  old tree** and fails there.
- **`tests/play.rs::a_shrunken_table_still_drops_pickers_after_later_frames`** — the case change-detection could wrongly
  skip: the write in one frame, plays in later ones.
- Nothing asserts a *choice*: the fix must not move the stream, and every existing determinism test is the guard.
