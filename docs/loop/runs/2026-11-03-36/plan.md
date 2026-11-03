# plan.md — iteration 36: the panel shows the score

## The pick, and why it beats the runner-up

**Pick:** `MusicState`'s seven panel-facing accessors are drawn by **nothing**, while the example ships a two-layer score
whose `Layer` step really crossfades on the bar — so the picture shows the music as two ordinary cues and never the
transition. Deferred **four times** for a budget that is now measured: 12 px free, 68 px needed for a heading plus a line.

Beats the runner-up (a `Stopped` line for a parent's despawn): that is blocked on the public-enum decision seven items wait
on. This one is unblocked, it is the crate's headline feature finally visible in its own showcase, and the budget arithmetic
is already in hand.

## The design

1. **A `Music` block** between `Buses` and `Cues`: `music.names()`, each with `layer_db`, plus `-> target_db` while
   `moving()`. One line for the whole score.
2. **The cue list filters out a cue the score owns** (`MusicState::score.layers[].cue`), so a layer is shown once, where it
   can show live gain. `music.stinger` stays: it is a one-shot on `Music`, not a layer.
3. **Seven log lines**, because the block costs 68 px against the 63 px it frees.

## Fields

None. No `src/` change, no public type, no schema; `MusicState` is read, not extended.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **The still, looked at:** `Music / calm -4.0 dB   combat -80.0 dB`, seven cue rows, `Problems / none`, log ending cleanly at
  **11 px** of margin.
- **A mid-crossfade capture, looked at:** `calm -79.9 -> -80.0   combat -3.1 -> -3.0` — the arrow is the score moving.
- **A capture at rest after the bar:** `calm -80.0 dB   combat -3.0 dB` — the crossfade landed.
- **The pixel measurement**: lowest ink `y = 709`, margin **11 px** (was 12 before the block, 1 with the block and eight
  lines).
- `check_media.py` after regenerating both images: 0 problems.

## The carve

- **Not** a row per layer: measured at 1 px margin, a clipped glyph — iteration 32's defect returned.
- **Not** eight log lines: same reason, measured both ways.
- **Not** `deepest_duck` or `next_change_s` in the header: the block shows the layers, and a header that repeats them is
  noise.
