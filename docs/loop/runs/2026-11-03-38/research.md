# research.md — iteration 38: the header counted a number the panel no longer showed

## The item

The panel's header read:

```
bevy_siren: 9 cues, 4 buses, 4 voices
```

and beneath it the cue list showed **seven** rows. Iteration 36 moved the score's two layers into a `Music` block and the
header kept `table.cues.len()` — the **table's** count, not the **panel's**. A reader who counts the rows under `Cues` gets
7, and the header says 9. The two moved cues are visible in the `Music` block, so the number is *arithmetically* right and
*reads* as a lie: 7 rows + 2 layers is 9, but nothing on screen says to add them.

That is the same class as iteration 19's `bus_db` staleness and iteration 31's `duck -8.0 dB` without a cause: **a number
on screen that the reader cannot check against the thing it names**.

## Sources

**S1. `MusicState::names` and `MusicScore::layers`** — the block shows the score's layers from `names()`; the cue list filters
`table.cues` by `music.score.layers[].cue`. The two sets are **disjoint by construction**, so the header's single number
cannot describe both.

**S2. Iteration 36's fix** — the block was built by removing cues from the list; the header was not part of that change, and
nothing tests a header.

**S3. `docs/media/README.md`'s alt text** — it says the still shows *"the nine shipped cues"*, which was true before the block
and is not now. A second place the same count lives.

**S4. The skill's step 6, pass 3** — *"can a developer understand the panel without the code?"* A header whose number cannot
be found below it fails that question in the smallest possible way.

## The technique

The header names the **split**: `{cues} cues, {layers} layers, {buses} buses, {voices} voices`, where
`cues = table.cues.len() - layers` and `layers = music.score.layers().len()`.

- **`saturating_sub`**, because a layer may name a cue the table does not have — that case is **reported** by
  `problems()`, and the header must not underflow while saying so. Measured: `1 cue, 2 layers -> (0, 2)`, no panic.
- **No score is `0 layers`**, and 7 + 0 + 4 + 4 is what a project without music sees.
- **Every number is now checkable**: the cue count against the `Cues` rows, the layer count against the `Music` block, the
  bus count against the `Buses` rows, the voice count against the sum of the bus rows' live counts.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the cue list and the layer block are disjoint | — | S1 |
| the block was added without the header | — | S2 |
| the alt text carries the same count in prose | "nine shipped cues" | S3 |
| a panel's numbers must be checkable against its rows | — | S4 |
| a layer may name a cue the table lacks — reported, not a panic | — | measured |

## What good sounds like

A header that reads `7 cues, 2 layers, 4 buses, 4 voices` over seven cue rows, a two-layer `Music` block, four bus rows and
four live voices — each number findable beneath it. With the fault table, `9 cues, 2 layers` over nine rows. No sound
changes.
