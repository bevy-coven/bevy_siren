# plan.md — iteration 51: a picker's window is what the cue asked for, not what its first play sized

## The pick, and why it beats the runner-up

**Pick:** `SirenChoices::pick` fixes each cue's `VariantPicker` capacity at **first play** (`or_insert_with`), so a cue
whose `avoid` grows afterwards silently keeps the old window. Measured: `capacity 1, avoid 3` and `capacity 1, avoid 1`
produce **byte-identical** 24-pick sequences.

Beats the runner-up (`CueProblem` coverage for a thin `avoid` window): that is a report needing the open
`bad-gain-problem.md` enum decision; this is a **behaviour bug** — rank 1 against rank 3 — and it needs no API break.

## The ranked list

1. **BUG — the picker's window is frozen at first play.** Shipped this iteration.
2. `CueProblem` for an `avoid` window that cannot be honoured — `pick_from`'s comment already promises `problems()`
   names it, and it does not. Needs the open enum decision; recorded below as a backlog item.
3. `AvoidRepeat`'s thin-table case is unreported (same decision).
4. `reset_histories` has **no caller anywhere** — fixed in the Cauldron (its own commit).
5. The `Duck::new` zero-hold/zero-fade item on iteration 50's list was **already shipped** (`problems()` reports it,
   `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time` exists). The list was stale; corrected here.

## The design

- **`VariantPicker::set_capacity(n)`** — grow or shrink the remembered window. **Additive public method**, so no
  consumer breaks. Growing allocates; shrinking keeps the most recent picks, which is the right half to keep.
- **`SirenChoices::pick`** calls it before `pick_from`, so `avoid` is read fresh from the cue and honoured.
- **`VariantPicker::new(capacity)`** keeps sizing both deques, so a caller that sets the capacity once still pays one
  allocation.

## Fields

**No new field.** No new `serde` type. The window is the picker's own memory, not an authored number.

## Tests

- **`tests/model.rs::a_raised_window_is_honoured_on_the_next_pick`** — a `capacity 1` picker asked `avoid: 3` over four
  keys must produce the `new(3)` sequence, and must differ from the `avoid: 1` one.
- **`tests/model.rs`** — `set_capacity` growing keeps the newest picks; shrinking keeps the newest, not the oldest.
- **`tests/play.rs::a_table_reload_that_raises_avoid_takes_effect`** — through `SirenChoices`, the public path.
- **Determinism**: the same seed and spawn order still repeat field for field (existing test, must stay green).
