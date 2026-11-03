# Iteration 105 — research: a repeat rule that refuses nothing

## The item

Not an open issue: a defect in the **cue table's repetition rule**, found by reading `Repeat::avoid()` against
`VariantPicker::pick_from`'s refusal arithmetic.

## The defect, measured

```rust
// cue.rs
pub fn avoid(&self) -> u8 {
    match self {
        Self::AvoidRepeat { avoid } => *avoid,   // returned verbatim
        Self::Shuffle => 0,
    }
}

// model.rs, pick_from
let refused = usize::from(avoid).min(len);
let recent: Vec<&str> = self.heard.iter().rev().take(refused).map(String::as_str).collect();
```

`AvoidRepeat { avoid: 0 }` therefore takes **zero** recent sounds and refuses none — **byte-identical to `Shuffle`** —
while the variant's own doc says *"Random, but never one of the last `avoid` variants — the default, and what kills the
machine-gun footstep"*.

Proved both ways:

- `a_zero_avoid_window_refuses_exactly_nothing` — two pickers, the same seed, 24 draws: the `avoid: 0` sequence and the
  `Shuffle` sequence are **equal**, and the `avoid: 0` sequence **contains a back-to-back repeat**. The `avoid: 1`
  sequence contains none.
- `avoid_repeat_zero_is_reported_because_it_refuses_nothing` — `problems()` now names it, and **does not** for `avoid`
  `1`, `2`, `4`, `255`, nor for `Shuffle`.

## Why this ranks first

`cues.yaml` case 1 is *"Variants do not repeat back to back"* and case 2 is *"A window wider than the variant list still
refuses the pick immediately before"*. A cue written `avoid: 0` fails case 1 and **looks perfectly normal while doing it** —
the play succeeds, no refusal is raised, and only a listener hears the machine-gun footstep. That is the shape the loop
ranks above everything else: silent wrong output.

**How a table reaches `avoid: 0`.** A generated table computing the window from a variant count — `avoid: variants - 1`,
which is `0` for a single-variant cue — produces it mechanically. Nothing warns.

## Sources

| Source | What it reports |
|---|---|
| Gregory, *Game Engine Architecture* 3e §14.5.8 | repetition control is a property of the **variant set**, not of a count of indices: a table's rule is about what a player hears |
| `cues.yaml` cases 1 and 2 | *"No two adjacent sounds are equal, and all three variants appear"*; *"A window wider than the variant list still refuses the pick immediately before"* |
| `VariantPicker::pick_from`'s doc | the avoided window is made of **keys, not indices** — *"two slots naming one file are one candidate for repetition"*: the crate already reasons in sounds, which is why a count of 0 is a count of nothing |
| Sun et al., `10.48550/arXiv.2406.19228` | **failure detection** and **fault assignment** — a rule that is inert must be named, not silently satisfied |

## The technique

`CueProblem::InertAvoidRepeat`, reported by `problems()`, with a sentence that says what happened and both remedies:

```
cue `footstep.grass` uses `AvoidRepeat` with `avoid: 0`, which refuses nothing — it plays like `Shuffle`, so the same
sample can repeat back to back. Set `avoid` to 1 or more, or use `Shuffle` and say so
```

**Why not make `avoid: 0` mean 1.** That would silently change a table's behaviour, and the crate's rule is that a bad cue
is *refused with a reason, never played wrong* — but this is a diagnostic, not a play path, so the honest repair is to
**report** and let the author choose. A `problems()` entry is the crate's existing tool for *"legal but almost certainly
not what you meant"*, as `UnlimitedInstances` already is.

## Parameters

None. This changes no rate, curve or threshold.

## What good sounds like

A table with `avoid: 0` gets one sentence on the editor's status line, naming the cue and both ways to fix it. A table with
`avoid: 1` gets nothing, because it is correct.
