# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.84 / 5.29 / 6.04**; disk 893 MB free, 100 % used.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.063 | 0.0079 |
| 64 | 0.354 | 0.0055 |
| 256 | 1.343 | 0.0052 |

**The change is not on this path.** `perfcheck` drives `model`'s pure arithmetic; this iteration added one `bool` to
`Started`, computed once in `answer` from an expression it already evaluated, and read twice in `spawn_voice`. A frame
with no play request executes none of it, and a frame with one executes two comparisons more than before. At 256
voices the whole figure is 1.343 µs of a 16.7 ms frame (0.008 %), and the load is 4.8 rather than iteration 7's 4.3 —
the difference between the runs is the machine.

## What *is* measured

1. **The panner's own law**, from the crate's own `model::backend_spatial_gain` at 8 m on-axis, now printed by
   `examples/render`:

   ```
   panner: its own law at 8 m on-axis is 0.0117 linear, -38.7 dB; a spatial cue divides it out,
   a non-spatial cue is never placed
   ```

   and pinned by `tests/model.rs::the_panner_law_is_what_a_spatial_cue_divides_out`.

2. **The routing decision**, by `tests/play.rs::a_non_spatial_cue_ignores_a_position`: the voice has no `SpatialListener`
   routing (`PlaybackSettings.spatial == false` on Bevy's path), no `Transform`, and `Played.gain_db` equal to the gain
   the sample actually plays at. On the unfixed tree the same test panics at *"a non-spatial cue is not routed through
   the panner"*.

**The render line reads the same in both trees on purpose** — it computes the law, not the decision. It is evidence of
the magnitude; the test is evidence of the fix. Confusing the two is the trap this iteration writes down.
