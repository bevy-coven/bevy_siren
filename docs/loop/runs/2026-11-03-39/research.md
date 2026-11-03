# research.md — iteration 39: the panel had never shown a refusal

## The item

The panel's own claim is *"the log of what played and why"*. A `Refused` is the **why** that matters most — the skill's
words are *"a refusal is the most valuable bug of all, because it is silent: a cue that never plays and never says why is
the hardest thing in a game to find"* — and the scripted run **never refused anything**:

```
Step::Steal   plays engine.idle ×2 and sword.hit  -> all three play (one steals)
Step::Duck    plays dialogue.greeting             -> plays
Step::Layer   writes MusicLayers                  -> moves
```

So the `Log` block drew `Played`, `Stopped` and `Ducked` lines and never a `Refused` one. The panel that exists to answer
*"why is the music quiet"* had never shown the answer to *"why is there no sound at all"*.

## Sources

**S1. The skill's ranking** — *"a refusal is the most valuable bug of all"*. The same reasoning applies to the panel: a
`Refused` line is the line a developer opens the panel for.

**S2. `Entry::line()`'s own doc** — *"The line a panel shows: one sentence, no formatting left to the caller."* The
`Refused` arm reads `{at_s:.2}s refused \`{cue}\`: {reason}` — already written, never drawn.

**S3. Iteration 30's `Problems` block** — the panel shows a **report** of table faults; a refusal is the runtime
counterpart, and the two had never appeared together.

**S4. The capture's own budget** — the fault table costs **three log lines** (two cue rows and two problem sentences), so a
panel with faults shows fewer log lines. Measured: seven leaves **1 px** of margin, five still **1 px**, **four restores
11 px**.

## The technique

1. **A `refuse` step** in the script: `Play::cue("no.such.cue")` — `Refusal::UnknownCue`, the clearest refusal there is,
  with the cue's own name as the reason.
2. **It joins the default script**, after `duck`, so `cargo run --example mixer` with no environment shows it.
3. **The log's line count follows the `Problems` block**: seven lines when the table is clean, **four** when it has faults —
  because the block's height is a function of the table it describes.

**What the capture proved, and what it did not.** The **still** at frame 95 opens its log with
`0.83s refused \`no.such.cue\`: no cue of that name` — verified twice by reading it. The **GIF** starts at frame 88, before
the refusal, so its first frame shows only `played`/`ducked` lines. That is a **capture-timing** limitation, not a panel
one: the refusal is visible for the ~30 frames a seven-line log takes to fill, and a GIF that starts before it does not
contain it. Recorded rather than hidden.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a refusal is the most valuable line to show | — | S1 |
| `Refused`'s line is already written for a panel | — | S2 |
| the panel already reports table faults | — | S3 |
| the fault table costs three log lines | 7 -> 4 | measured |
| the refusal is visible for | ~30 frames | measured |

## What good sounds like

A panel whose log opens with `refused \`no.such.cue\`: no cue of that name` — the reason a person opened it — beside a
`Problems` block that shows the same discipline for the table: both name what is missing, in the crate's own words. No
sound changes.
