# review.md — iteration 30, three passes

## Pass 1 — function

**The claim.** Iteration 26's `Problems` block grew without limit in a **fixed, unscrollable column** and sat **above** the
log — the section that answers *"why is the music quiet"*. With six faults the log left the screen.

| state | source | before | after |
|---|---|---|---|
| six faults, 1280×720 | S1 (the panel promises the log), S3 (the log is what a person reads when something is wrong) | six sentences; the `Log` heading and its ten lines **off the bottom** | three sentences, `and 3 more; the log below is what played`, **log on screen** |
| two faults (the README's GIF) | S2 (the log block is bounded at ten; the panel bounded one list and not the other) | both sentences, log present | **unchanged** — two fits under the bound |
| clean table | S2 | `Problems` / `none` | unchanged |
| every sound | — | — | **byte-identical** (21/21) |

**Against the sources.** S2 is the inconsistency: the panel already bounded the log at `recent(10)` and did not bound the
block above it. A view that drops its most important section when it has the most to say is the opposite of what an
editor panel is for.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{many-faults,many-faults-after,two-faults-after,clean-after,zoom-problems-many}.png`, and the
regenerated `docs/media/mixer.{png,gif}`.

**A defect in my own first version, caught by looking.** The count line used an em dash: `and 3 more — the log below is what
played`. The captured frame showed a **box** — `and 3 more ▯ the log` — because the panel's font has no glyph for `—`. A
debug view cannot afford a character the font may not have; it is ASCII now.

**The zoom confirms the fix.** `zoom-problems-many.png` shows three sentences, `and 3 more; the log below is what played`,
and the `Log` heading directly beneath — the section that was missing.

**The bound does not fire when it should not.** `two-faults-after.png` shows both sentences with **no** count line and the
full ten-line log, because `2 <= SHOWN`. A cap that always printed `and 0 more` would have been a worse fix than none.

**Verdict: GREAT** after the glyph correction.

## Pass 3 — fresh eyes

**A different shape.** Three captures of one binary at three fault counts — six (the bound fires), two (it does not), zero
(`none`) — so the fix is seen **not** firing as well as firing. `SIREN_FAULTS=many` is a new knob beside the existing one,
documented in the module doc's capture list.

**Can a developer understand the panel without the code?** Better than before: the panel now shows all five blocks at
every fault count, and the truncation says how many are hidden and points at the log. Before, a developer with six faults
saw a panel that simply ended.

**What a fresh reader would still ask:** *why three?* Because three sentences plus the log's ten lines fit 1280×720 with
the bus rows and cue list above them — measured on the capture, the same way the log's ten were chosen. A taller window
could show more; the bound is for the size the harness captures.

**Verdict: GREAT.**
