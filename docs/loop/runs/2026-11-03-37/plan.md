# plan.md — iteration 37: the GIF samples a fade by the second, not by the update

## The pick, and why it beats the runner-up

**Pick:** `SIREN_SHOT_DIR` sampled every `SIREN_SHOT_EVERY`th **update**; a layer fade is 50 ms, about **6 updates**, and the
shipped GIF sampled every **12** — **0.5 samples per fade**. The `Music` block's `-> target` arrow, which iteration 36 built
the block for, appeared or did not by the luck of the sample phase.

Beats the runner-up (a `Stopped` line for a parent's despawn): that is blocked on the public-enum decision seven items wait
on. This one is unblocked, it makes the last iteration's feature **provable in the picture**, and it removes a magic
constant I had just introduced.

## The design

1. **`SIREN_SHOT_EVERY` becomes hundredths of a second** (12 = 120 ms). A duration for a duration.
2. **`capture` keeps a frame when** `time.elapsed_secs_f64() - kept_at_s >= every / 100`, and `Capture` gains
   `kept_at_s: f64`.
3. **The doc says why**: the knob used to be an update count, and an update is not a time.

## Fields

None on `CueTable`, `SirenSettings` or `MusicScore`. `Capture` is private to the example.

## Tests

The example has no unit tests in this repo's harness. What proves it:

- **A count:** `SIREN_SHOT_EVERY=4` (40 ms) kept **47 frames** and **11 consecutive** carried the arrow (`f_0028`–`f_0038`);
  the old sampling gave ~0.5 samples per fade on a 20-frame GIF.
- **A crop, looked at:** `evidence/panel/zoom-fade-31.png` reads
  `calm -76.2 -> -80.0   combat -6.9 -> -3.0` — the crossfade in flight.
- **The regenerated GIF**, 46 frames at 10 fps.
- Full gate; counts unchanged (119 / 126 / 128 + 5), because no `src/` line moved.

## The carve

- **Not** a hard-coded `8.333` ms-per-update conversion: my first version had one, and the crate's rule forbids a magic
  constant in a system. The knob is a duration instead.
- **Not** `next_change_s` in the panel: 11 px free, and the block's arrow now shows the same fact while it is happening.
