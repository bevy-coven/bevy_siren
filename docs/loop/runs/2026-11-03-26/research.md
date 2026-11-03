# research.md — iteration 26: the panel draws a broken cue exactly like a good one

## The item

`examples/mixer` is the crate's visible harness — the picture the loop now ships with a visible change. Its
`draw_panel` reads `SirenMix`, `CueTable`, `SirenSettings` and `SirenLog`. **`problems()` is not among them.** So a cue on
a bus the mix does not have, or one whose `pitch.low` is past `pitch.high`, is drawn with the same four columns as a
healthy one:

```
footstep.grass       Effects    once  +0.0 dB  priority 10
sword.hit            Reverb     once  -2.0 dB  priority 5      <- a bus that does not exist
bad.pitch            Effects    once  +0.0 dB  priority 0      <- pitch.low past pitch.high
```

Nothing marks it. A reader cannot tell that two of these cues can never play correctly — and the panel's whole reason
for existing is the editor's: **make a fault visible**.

The Cauldron's `audio_panel.rs` *does* chain `problems_in(&read)` into its status line, so the real editor has the
sentence. The crate's own example does not, and iteration 25's log recorded this as the next item.

## Sources

**S1. The module doc's own claim** — *"The audio panel, as a program: every bus with its level, its duck and its live voice
count; the cue table; the log of what played and why"*. Three of those four are drawn from the crate's own resources with
their real numbers; the fourth, the cue table, is drawn **without the crate's own verdict on it**.

**S2. R6 / the crate's fail-loud rule** — *"a bad cue is refused with a reason, never played wrong"*, and
`CueProblem`'s `Display` exists to be printed. The crate computes the sentence and the panel does not ask for it.

**S3. The skill's step 6, pass 3** — *"can a developer understand the panel without the code?"* A panel that draws a
broken cue as healthy **fails that question** in the worst direction: it says the cue is fine.

**S4. `iteration 25`'s review** — the panel passed pass 3 *for a clean table*; the question is what it does with a table
the crate rejects, which was never asked. That is this iteration.

## The technique

Two additions to the example, no public API change:

1. **A `Problems` block in the panel**, headed like the others, listing `table.problems_in(settings)` — one sentence per
   line, in `CueProblem::Display`'s own words, and **`none`** when the table is clean. Using `problems_in` (iteration 21's
   settings-aware call) rather than `problems()` matters here: the example's table has an `Ambience` bus and a `Music`
   bus, and `problems()` would report every cue on them as `UnknownBus` — the false report iteration 21 fixed.
2. **A marker on a bad cue's row**, so the fault is beside the thing it is about: a trailing `  <problem>` drawn from the
   same list, so a reader scanning the column sees which row is wrong without cross-referencing.

**A table with one deliberate fault in the capture.** The example's own table is clean, so the panel would show `none`
and the capture would prove nothing. The demonstration uses a **second table** in the evidence run (a bad bus and an
inverted pitch) rather than polluting the example's shipped table — the capture is what proves the block works, and the
example keeps its clean, legible screenshot.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the panel shows the cue table with its real numbers | — | S1 |
| `CueProblem`'s `Display` exists to be printed | — | S2 |
| a panel must explain itself without code | — | S3 |
| `problems_in(&settings)` is the settings-aware report | — | iteration 21 |

## What good sounds like

A panel whose cue list is followed by a `Problems` block: `none` for a healthy table, and for a bad one, one sentence per
line naming the cue and what is wrong — the same sentence the Cauldron's status line shows. A developer looking at a
capture can see at a glance that a cue can never play, without reading the table file.
