# review.md — iteration 39, three passes

## Pass 1 — function

**The claim.** The panel's claim is *"the log of what played and why"*. A `Refused` is the "why" that matters most, and the
scripted run never refused anything: `steal` steals, `duck` ducks, `layer` moves — every step **plays**.

| row | source | before | after |
|---|---|---|---|
| a refusal in the log | S1 (the skill: a refusal is the most valuable bug), S2 (`Entry::line` already writes it) | **never** — no step refuses | `0.83s refused \`no.such.cue\`: no cue of that name`, at the **top** of the log |
| the default script | — | `steal,layer,duck` | `steal,layer,duck,refuse` |
| the fault table's log | S4 (the fault table costs three lines) | 7 lines, **1 px — clipped** | **4 lines, 11 px** |
| the GIF's width | — | 960 (down-scaled) | **1280, native** |

**Against the sources.** S1 and S2 together: the line was **already written** by `Entry::line` and had never been drawn,
because nothing in the script produced one. S4 is a budget, measured three ways (7 → 1 px, 5 → 1 px, 4 → 11 px).

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/panel/{refusal-1280,zoom-refusal,zoom-log-95,faults-final,zoom-tail-faults}.png`, the still and the GIF.

**The still is the proof, and it is unambiguous:** at frame 95 its log opens with
`0.85s refused \`no.such.cue\`: no cue of that name` — the panel's "why" is the first line a reader sees.

**What I got wrong three times, and what settled it.** (1) I read a *clipped* last line in a crop, then measured the window
and found 11 px — the crop's edge, not the panel's. (2) I computed the fault table's overflow as −59 px from row heights
while the measurement said 1 px, so my arithmetic was the wrong model; the pixel scan decided it. (3) I assumed the GIF's
960-wide down-scale caused the wrap; it did not — the fault table overflows at **1280** too, and the fix was four log lines.

**The GIF's first frame is at 88, before the refusal at 90.** Recorded in `research.md` and here: the **still** proves the
line, the **GIF** proves the layout. Re-timing the GIF is a knob (`SIREN_SHOT_FROM`), not a defect — and chasing it cost
several captures, which is the honest account.

**Verdict: GREAT** after the three corrections.

## Pass 3 — fresh eyes

**A different shape.** Three states: the clean still at frame 95 (refusal newest), the fault table at 95 (four log lines,
11 px), and the clean GIF at 88 (seven log lines, native 1280). The fault table is what proves the line count is
**computed** — a hard-coded seven would pass the clean still and clip the fault one.

**Can a developer understand the panel without the code?** The log's first line is now the crate's own refusal sentence,
and the `Problems` block is its authoring counterpart. A person reading the panel sees both halves of the crate's promise:
*what is wrong with this table*, and *why nothing sounded*.

**What a fresh reader would still ask:** *why does the fault table show fewer log lines?* Because it costs three of them —
two cue rows and two problem sentences — and a clipped glyph is worse than two fewer lines. The comment says so, with the
three measurements.

**Verdict: GREAT.**
