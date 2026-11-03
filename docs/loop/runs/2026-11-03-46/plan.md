# plan.md — iteration 46: a duck with neither a hold nor a fade is reported

## The pick, and why it beats the runner-up

**Pick:** `Duck::new(a, b)` produces a rule that **ducks for zero seconds** — it fires, logs, and is inaudible — and the
nine-sentence report said nothing. Measured this iteration, not assumed:
`a_duck_with_no_hold_and_no_fade_ducks_for_no_time` gets `Some(0.0)` from `Duck::new("Dialogue","Music").by_db(-8.0)`
and `Some(-8.0)` from the same rule with `.hold_s(0.25).fade_s(0.6)`.

Beats the runner-up (a `MissingSound` refusal capture): that is a picture of something that already works; this is a
**silent authoring trap on the most natural constructor**, which is rank-1 by the loop's own ranking.

## The blocked item

**Case 31** (a non-finite cue `gain_db` reported by `problems()`) — still blocked on
`needs-decision/bad-gain-problem.md`, a public enum. Nothing built from it.

## Why this needs no decision file

`SirenSettings::problems` returns **`Vec<String>`**, not a public enum, so a tenth sentence is **additive**: no
consumer's `match` breaks. Iteration 36 relied on the same fact for the score's report. (A `CueProblem` or `Refusal`
variant would need one; this does not.)

## The design

1. **A tenth check** in `SirenSettings::problems`:
   `hold_s == 0.0 && fade_s == 0.0 && by_db < 0.0` → *"has no hold and no fade, so it lowers the bus for no time at
   all; set hold_s or fade_s"*. The `by_db < 0.0` guard keeps a **raising** rule from being reported twice — it already
   has its own sentence.
2. **The doc gains a tenth bullet**, and its "Nine checks" line says ten.
3. **`problems_names_every_mistake` gains the sentence**, and its big fault table now gives `hold_s` to the rules it
   had left bare — otherwise the new sentence fired from a rule whose real fault was a missing bus, and the assertion
   passed for the wrong reason. A dedicated `Duck::new("Music", "Music")` row carries it.
4. **`SIREN_MIX=broken` adds that shape**, so the panel draws all three of its faults.

## Fields

**No new field.** `hold_s`/`fade_s` already exist; this reports their combination.

## Tests

- **`tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time`** — the measured behaviour: `0.0` from the bare
  rule, `-8.0` with a hold; and a good table still reports nothing.
- **`problems_names_every_mistake`** — the tenth sentence, attributable, mutation-checked (`if false` reddens it).
- Two shadowing/typing mistakes in my own test were caught by the compiler and fixed; recorded because the first
  version asserted `None` where `ducked` holds every bus at `0.0`.

## The carve

- **Not** changing `Duck::new`'s defaults to match `default_ducks()`. That is a **behaviour change** to every project
  that writes a bare rule today — some may rely on a rule that fires and does nothing — and it belongs in a decision
  file, not a patch. The report names the mistake instead, which is the crate's rule ("a bad cue is refused with a
  reason, never played wrong" — here named, not refused).
- **Not** a sentence for a **self-duck** (`Effects → Effects`): measured while investigating, it is legal and does what
  it says; whether it is a fault is a separate question and remains in the backlog.
