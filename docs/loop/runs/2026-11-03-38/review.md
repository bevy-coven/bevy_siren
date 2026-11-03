# review.md — iteration 38, three passes

## Pass 1 — function

**The claim.** The header read `9 cues` and the cue list showed **7**. Iteration 36 moved the score's two layers into a
`Music` block and left the header counting `table.cues.len()` — the table's number, not the panel's.

| row | source | before | after |
|---|---|---|---|
| the still's header | S1 (the cue list and the layer block are disjoint), S4 (every number checkable against its rows) | **`9 cues`** over **7** rows | **`7 cues, 2 layers`** over 7 rows and a 2-layer block |
| the fault table's header | S3 | `9 cues` — the table's count, which *happened* to match its 9 rows | `9 cues, 2 layers` — still 9 rows, and the layers named |
| a table with fewer cues than layers | — | would have underflowed | `saturating_sub`: `1,2 -> (0,2)`, no panic |
| every sound | — | — | **byte-identical** (21/21) |

**Against the sources.** S4 is the whole argument: the panel exists so a person can read it, and a header whose number
cannot be found by counting the rows beneath it is the smallest possible version of that failure. S3 is the second copy of
the same count — the alt text says *"the nine shipped cues"* — recorded in `research.md` as still true of the **table**, and
now no longer claimed of the panel.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{header-1280,zoom-header-faults}.png`, the regenerated `docs/media/mixer.{png,gif}`, and a pixel
scan of the still.

**Both states checked, and the second is the one that proves it is computed.** The clean still reads `7 cues, 2 layers`;
the fault-table capture reads `9 cues, 2 layers` over **nine** rows (7 shipped + 2 faults). A hard-coded `7` would have
passed the first and failed the second.

**The arithmetic's edges probed rather than assumed:** `9,2→(7,2)`, `2,2→(0,2)`, `1,2→(0,2)` — a layer naming a cue the table
lacks, which `problems()` reports and which must not underflow — and `0,0→(0,0)`. No `src/` change, so nothing else could be
affected.

**Margin unchanged at 11 px.** The header gained two words and no line.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape.** Two captures of one binary, differing only in `SIREN_FAULTS`, so the header is seen computing rather
than appearing — and the fault capture has a *different* correct answer, which is what a hard-coded value cannot give.

**Can a developer understand the panel without the code?** The header's four numbers now map one-to-one onto the four blocks
below: `7 cues` → the seven `Cues` rows, `2 layers` → the `Music` block, `4 buses` → the four `Buses` rows, `4 voices` → the
sum of their live counts. That is the cleanest pass-3 answer the panel has had.

**What a fresh reader would still ask:** *why not "9 cues (2 layers)"?* Because that re-invents the addition for the reader;
the panel's job is to say what it shows, and it shows the two separately.

**Verdict: GREAT.**
