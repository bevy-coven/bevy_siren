# research.md — iteration 37: the GIF sampled a fade by the wrong unit

## The item

`SIREN_SHOT_DIR` kept every `SIREN_SHOT_EVERY`th **update** for a GIF. A layer fade is `MIX_TAU_S` = **50 ms**; the example
runs at ~8.3 ms/update, so a fade lasts **6 updates**. The shipped GIF sampled every **twelve** updates:

```
6 updates per fade / 12 updates per sample = 0.5 samples per fade
```

**A coin flip.** The `Music` block's `-> target` arrow exists only while `moving()`, so whether the showcase's crossfade
appeared at all depended on the phase of the sample — and the log's own `stopped`/`ducked` lines move at the same rate.

Measured on the shipped GIF before this change: 20 frames, and the music line's ink changed only across frames 12–16 — it
**did** catch the fade, by luck of the phase. A showcase that is right by luck is not a showcase.

## Sources

**S1. `MIX_TAU_S`'s doc** — *"a gain moves at this rate so a bus does not click"*, 50 ms. The fade is a **duration**, and the
only honest unit for sampling it is a duration.

**S2. `SIREN_SHOT_EVERY`'s doc** — *"also keeps every `SIREN_SHOT_EVERY`th frame"*. A frame is an **update**; the thing being
captured is a fade. The knob's unit and its purpose disagreed.

**S3. Iteration 36's block** — the `-> target` arrow is the whole reason the `Music` block exists, and it is visible only
during a fade. A GIF that samples the fade at half a sample cannot show the feature the last iteration built.

**S4. The crate's "no magic constant" rule** — my first fix multiplied `every` by a hard-coded `8.333` to turn updates into
milliseconds. That is exactly the constant the rule forbids: the knob should **be** a duration, not a count converted by a
number nobody can source.

## The technique

`SIREN_SHOT_EVERY` becomes **hundredths of a second** (12 = 120 ms), and `capture` keeps a frame when
`time.elapsed_secs_f64() - kept_at_s >= every / 100`. `Capture` gains `kept_at_s: f64`.

- **The knob's unit is now the knob's purpose.** A duration for a duration.
- **The GIF's frame rate is the capture's frame rate**, not a function of the machine's speed — an 8.3 ms/frame Mac and a
  16 ms/frame Mac now produce the same *temporal* samples, which is what a GIF is.
- **The default stays 2** (20 ms), which is four times denser than the fastest fade the crate has.

**The proof is a count, not an impression:** at `SIREN_SHOT_EVERY=4` (40 ms, denser than a 50 ms fade) the capture kept
**47 frames** and **11 consecutive frames** carried the arrow (`f_0028`–`f_0038`), where the old sampling gave ~0.5.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| a layer fade lasts `MIX_TAU_S` | 50 ms | S1 |
| the example runs at ~8.3 ms/update | ≈120 updates/s | measured |
| a fade is therefore ~6 updates | 6 | measured |
| sampling every 12 updates | **0.5 samples per fade** | measured |
| sampling every 40 ms | **11 frames with the arrow** | measured |

## What good sounds like

A GIF in which the score's crossfade is **visible for a stretch of frames** — `calm -76.2 -> -80.0   combat -6.9 -> -3.0` —
rather than a single frame that may or may not be there. Nothing about the panel's content changes; the capture now samples
the thing it claims to show.
