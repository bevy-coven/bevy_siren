# review.md — iteration 44, three passes

## Pass 1 — Function: does it do what the source says?

**The source.** S1: Gregory, *Game Engine Architecture* 3rd ed. §14.5.8 — *"a **virtual voice** is a sound the game
believes is playing but that has no hardware voice"*. S2: `blocked_at`'s own doc — *"the live voice count at the frame
this layer's start was refused… a virtual voice is promoted when a voice frees"*.

**Measured** (`evidence/panel/tight-40.png`, 1280×720, `SIREN_TIGHT=1`, frame 40):

| Where | Text |
|---|---|
| `Music` block | `calm -4.0 dB   combat -80.0 dB **waiting**` |
| bus row | `Music -2.0 dB 1 / 1 voices` |
| log | `0.00s refused \`track.combat\`: no voice free on bus \`Music\`` |
| log | `0.00s played \`track.calm\` (music_calm) on Music at -4.00 dB` |

One voice, two layers, and the panel says which one has it — the block, the bus count and the log agreeing. At 1.18 s
with seed 777 and a different script (`tight-fresh-140.png`) the same line holds.

**The assertion, before and after.** With the budget full: both layers `Some(1)`. After one slot frees:
`(calm: None, combat: Some(1))` — the layer that got a voice stopped waiting, the other still is. A layer the score does
not have answers `None`.

**Verdict: GREAT.** The state existed and was correct; the reading was missing, and now two places state it.

## Pass 2 — Zoom: read the crops closely

Read: `evidence/panel/{tight-40,tight-40-1920,tight-fresh-140,zoom-waiting}.png`, and the shipped
`docs/media/mixer-waiting.png`.

**A real defect this pass found.** The first `SIREN_TIGHT` set the **table's** `max_voices` to 1. The capture then showed
the waiting marker — but also six `refused` lines for every other play in the script, and every bus at `0 / 1`, so the
picture proved the marker by silencing the panel. Scoping the cap to the `Music` bus fixed it: `Effects 2 / 2`, the bus
steal intact, and the *only* music lines are the two that matter. A demonstration that starves what it demonstrates is
not a demonstration.

**A second thing, checked.** At frame 40 the `Music` block's other layer reads `calm -4.0 dB`, which is its **live** gain
against an authored `-4.0` — the layer started at its score gain, so live equals authored here by construction. The
block's `-> target` form would show if they differed; it does not, correctly.

**No clipping, no wrap.** At both 1280 and 1920 the block is one line of 41 characters and the four bus bands stay
evenly spaced. The still keeps its 9 px margin.

**Verdict: GREAT.** After the fix the picture isolates the waiting layer rather than starving the panel.

## Pass 3 — Fresh eyes: a different script, a different seed

- `SIREN_TIGHT=1 SIREN_SEED=777 SIREN_KEYS=duck`, frame 140 — the waiting marker is unchanged, because it is a property
  of the **budget and the score**, not of the run. The log's `refused \`dialogue.greeting\`: no voice free` lines appear
  only when a duck step runs, as the script intends.
- The default table (`SIREN_TIGHT` unset) shows **no** `waiting`, because both layers get a voice. The marker appears
  only when it is true, which is the property that keeps it from becoming noise.
- A layer the score does not have answers `None`, asserted.

**Can a developer understand it without the code?** Yes. `waiting` is an ordinary English word for the state, the bus
row's `1 / 1 voices` is the arithmetic that produced it, and the log's `refused … no voice free on bus \`Music\`` is the
sentence form of the same fact. `docs/guide/music.md` names Gregory's term in the same place.

**Verdict: GREAT.**
