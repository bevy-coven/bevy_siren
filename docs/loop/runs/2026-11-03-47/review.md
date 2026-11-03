# review.md — iteration 47, three passes

## Pass 1 — Function: does it report the thing that happens?

**The measurement.** A rule `Duck::new("Effects","Effects").by_db(-6.0).hold_s(0.25).fade_s(0.4)`, a voice starting on
`Effects`:

| | `SirenMix::ducked["Effects"]` | `bus_db("Effects")` |
|---|---|---|
| no voice | `0.0` | 0 dB |
| a voice starts | **`-6.0`** | **6 dB lower** |

And the panel shows it running: `0.38s sword.hit ducked Effects by -6.0 dB` — the rule firing, on the bus its own
voice plays on.

**After.** `problems()` emits an eleventh sentence; the capture draws it:
`the duck on \`Effects\` starts on the bus it lowers, so every voice on it quietens itself`.

**Verdict: GREAT.** The behaviour is measured, unchanged, and now named where every other table fault is.

## Pass 2 — Zoom: read the crop closely

Read: `evidence/panel/broken-40.png` at each of three revisions.

**A defect found and fixed by looking.** The first sentence was **118 characters** and wrapped onto a **second line** —
`usually different buses` sat alone under it — which pushed the block to four rows and the log to `and 1 more`. Shortened
to **78 characters**, it fits one line, `and 1 more` bounds the third fault, and three log lines are visible.

**A second defect, in my own panel table.** `SIREN_MIX=broken` still carried iteration 45's two ducks because
`cargo fmt` had reflowed the block and my replacement silently did not match. The capture showed the *old* sentences and
I nearly read it as "the new sentence does not fire" — the file, not the code, was wrong. Fixed by reading the block and
replacing it exactly.

**A third, caught by the compiler twice**: `let mut app = app(...)` shadowed the helper, and the first version asserted
`None` where `SirenMix::ducked` holds every bus at `0.0`.

**Verdict: GREAT.** After the two fixes the block is one sentence per row and the log is intact.

## Pass 3 — Fresh eyes: the report against two tables

The panel was read with the **clean** table (`Problems: none` — the control: the new sentence appears only for a rule
whose ends coincide, so it cannot become noise) and the **broken** one. The crate's own `default_ducks()` — Dialogue →
Music and Dialogue → Effects — has different ends on both rules, so the pair every project starts from reports nothing.

**Can a developer understand it without the code?** Yes. The sentence names the mechanism (`starts on the bus it
lowers`), the consequence (`every voice on it quietens itself`), and does not scold: a project that meant a compressor
reads a true sentence.

**Verdict: GREAT.**
