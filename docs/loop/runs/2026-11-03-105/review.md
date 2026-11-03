# Iteration 105 — review

## Pass 1 — function

**The defect.** `Repeat::AvoidRepeat { avoid: 0 }` refuses nothing: `Repeat::avoid()` returns the field verbatim and
`pick_from` takes `min(avoid, len)` recent sounds, so zero is **byte-identical to `Shuffle`** while the variant's doc
promises *"never one of the last `avoid` variants — the default, and what kills the machine-gun footstep"*.

| criterion | evidence |
|---|---|
| the defect is real, not a reading error | `a_zero_avoid_window_refuses_exactly_nothing` — two pickers, one seed, 24 draws: the `avoid: 0` and `Shuffle` sequences are **equal** |
| it produces the outcome the rule prevents | the same test asserts the `avoid: 0` sequence **contains a back-to-back repeat**, and that `avoid: 1` does not |
| it is reported, not silently repaired | `avoid_repeat_zero_is_reported_because_it_refuses_nothing`; the fix does **not** coerce `0` to `1` |
| the refusal that must not fire does not | `avoid` of `1`, `2`, `4`, `255` and `Shuffle` all report nothing |

**Why report rather than repair.** The crate's rule is *"a bad cue is refused with a reason, never played wrong"*. This is
not a play path — the table is legal — so the existing tool is `problems()`, as `UnlimitedInstances` already is for a
`0` that means "no limit". Coercing `0` to `1` would change a table's behaviour without saying so, which is the failure
mode `avoid: 0` already is.

**Against the cue cases.** Case 1 requires *"no two adjacent sounds are equal"* for `avoid_repeat: 1` — green, and this
defect is its blind spot: the same table with `0` satisfies neither the case nor its intent, and nothing said so.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/footstep-grass.png` and the loudness figures.

- `footstep-grass.png`: a 0.025 s attack across the full band, a low-frequency body to 0.03 s, then a flat floor to 0.33 s.
  **No vertical line**, no shelf, no hole. A footstep is exactly the clip where a click shows, which is why it is the one
  rendered.
- **All 21 rendered wavs byte-identical** to iteration 104; master **−21.5 LUFS, −7.5 dBFS true peak**.

Nothing audible could change — the fix is a diagnostic — and the byte comparison is the stronger statement.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

Read as a table author whose generated file emitted `avoid: 0`:

- The sentence names the cue and **both** remedies: *"Set `avoid` to 1 or more, or use `Shuffle` and say so"*. The second
  matters because a table that genuinely wants repeats is legal and now has a name for what it is doing.
- The `Repeat::AvoidRepeat` doc now says `avoid` counts **sounds**, not slot indices — which is the other half of the same
  confusion, and it was already true of `pick_from`'s key-based window.
- Read as a maintainer: `problems()` is where this lives, so a panel already showing `CueTable::problems()` displays it
  with **no UI work** — that is the Cauldron criterion met by construction.

**One thing this iteration forced, and it is the criticism worth recording:** adding the variant **broke `tests/table.rs`'s
exhaustive `match`** — `error[E0004]` — in a test whose subject is *"each problem is a sentence naming its cue"*, not
*"the set of problems is exactly these six"*. The closure now reads the name and does not enumerate, which is what it always
meant. That is the **fourth** diagnostic enum here to make a consumer pay for a new variant, and it is why
`needs-decision/diagnostic-enums-are-extensible.md` now proposes `#[non_exhaustive]` for the print-only enums and a written
rule for the difference.

**A second self-inflicted lesson:** I first added an `InertAvoidRepeat` row to `problems_names_each_mistake`'s expected
list, but that test's fixture has no cue with `avoid: 0`, so the assertion failed under `--all-features`. The assertion
belongs to its own test, which already has it. Asserting a problem the fixture does not produce is the same class of
mistake as iteration 104's — a test making a claim about data it did not create.

**Verdict: GREAT.**
