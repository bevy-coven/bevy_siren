# plan.md — iteration 26: the panel shows the crate's own verdict on its table

## The pick, and why it beats the runner-up

**Pick:** `examples/mixer`'s panel draws the cue table with its name, bus, mode, gain and priority — and **not**
`problems()`. A cue on a bus the mix does not have is drawn exactly like a healthy one, so the picture that exists to be
judged says a broken cue is fine. The crate already computes the sentence (`CueProblem::Display`) and the Cauldron
already shows it; the crate's own example does not.

Beats the runner-up (the four blocked `CueProblem` variants): those need a public-enum decision the owner has not
answered. This one is unblocked, is the panel's stated job, and is the first iteration in which the loop can **show** a
fault in a screenshot rather than describe it.

## The design

In `examples/mixer.rs`:

1. **`Kind::Problems`** — a fourth block, headed `Problems`, listing `table.problems_in(&settings)` one per line, or
   `none`. `problems_in` (iteration 21) rather than `problems()`: the example's table uses `Ambience` and `Music`, and
   `problems()` assumes the crate's default four — the false report iteration 21 fixed.
2. **A trailing marker on a bad cue's row**, taken from the same list, so the fault sits beside the cue it is about.

`draw_panel` gains one resource read (`settings` is already there) and one match arm. No `src/` change, no public type, no
schema.

## Fields

None. Example only.

## Tests

The example has no unit tests in this repo's harness (it is a binary driven by env vars). What proves it:

- **A capture of a clean table**: `Problems` reads `none`, and the picture is otherwise unchanged.
- **A capture of a table with two faults** (a cue on `Reverb`, a cue with `pitch.low > pitch.high`): two sentences, one
  per line, in `CueProblem`'s own words; and the two cue rows carry their markers.
- **Both looked at** at 1280×720, per the skill's step 6.
- `cargo test` unchanged — nothing in `src/` moves.

## The carve

- **Not** the example's shipped table: it stays clean, because its screenshot is the crate's face and a deliberate fault
  in it would be a worse picture. The fault table is a capture-only table, selected by a new env knob
  (`SIREN_FAULTS=1`), documented in the module doc beside the other knobs.
- **Not** a colour or icon: the panel is monospace text with no style vocabulary, and a word is legible in a PNG at
  1280×720 where a colour shift is not.
