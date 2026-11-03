# plan.md — iteration 41: the panel shows a placed sound

## The pick, and why it beats the runner-up

**Pick:** `examples/mixer` had **no spatial cue and no `.at(..)`**. Cases 13 and 14 — the refusal with no listener, and
distance deciding gain — were tested and never drawn, so the crate's headline spatial feature (three attenuation models,
a panner, a refusal class) was invisible in its own showcase. This iteration adds the placed cue, the step, the row that
shows the law, and the precision fix that lets the row and the log state one number.

Beats the runner-up (`max_instances` as a cue-list column) because that is rank 3 ergonomics while placement is rank 2
core behaviour — and because the row already had the bug class this loop keeps fixing: **the panel could not show a gain
it computed**, since the log printed one decimal less than the row.

## The design

1. `Cue { sounds: ["hit_01"], bus: "Effects", priority: 30, spatial: true, ..Cue::new("bang") }`.
2. `Step::Spatial` → `Play::cue("bang").at(Vec3::new(0.0, 0.0, PLACED_M))`, `PLACED_M = 8.0` — 2 × the default `near_m`,
   the distance at which the law is exactly a halving.
3. The cue row appends `inverse at 8 m -6.02 dB`, computed by `model::distance_gain` + `model::gain_to_db` — the crate's
   own functions, not a copy of the formula.
4. `Entry::line`'s played gain: `{gain_db:.1}` → `{gain_db:.2}`.
5. Log lines: 7 → **6**. The eighth cue row costs 21 px; measured with seven, the lowest ink sat at `y = 719` of 719 —
   **0 px**, the defect iteration 32 fixed.

## Fields

**No new field on `CueTable`, `SirenSettings` or `MusicScore`.** `bang` uses `spatial: true` and the default
`Attenuation::Inverse { near_m: 4, rolloff: 1 }`, whose source is `docs/research/siren.md`. The example's `PLACED_M` is a
**design default** (8 m, chosen as 2 × `near_m` so the law shows a halving).

## Modules touched

- `examples/mixer.rs` — the cue, the step, the row, the log count.
- `src/log.rs` — **one format string**, `{:.1}` → `{:.2}`, in `Entry::line`. No type, field, variant or schema change;
  `gain_db` keeps full precision.
- `tests/model.rs` — `a_played_gain_is_written_to_two_decimals`.

## Tests

- `tests/model.rs::a_played_gain_is_written_to_two_decimals` — asserts `at -6.02 dB` **and** that the field keeps
  −6.0206. Mutation-checked: reverting to `{:.1}` reddens it.
- `tests/play.rs::a_spatial_play_attenuates_with_distance` — case 14's own assertion, re-run: 0.0 dB at 4 m, −6.0206 at
  8 m, green under both feature sets (120 / 127 / 129 + 5).
- `cues.yaml` case 13 stays covered by `tests/play.rs::a_spatial_play_with_no_listener_is_refused`, and the render harness
  writes `refusal-bang-no-SpatialListener-to-place-a-sound-at--3--0--4-.txt`.

## The carve

- **Not** a separate `Spatial` panel block: the panel is already 11 cues and six blocks, and a block would have cost three
  more log lines. The cue row is where a reader asks "why is that one quiet", so the law goes there.
- **Not** a panner readout: `src/mixer` does not pan (`docs/research/siren.md`, Deviations), so a pan column would show
  the same value on both backends and teach the wrong model.
- **Not** `SpatiallyMute` in the panel: the example always spawns a listener, and a capture that removes it would change
  every other block's numbers. The render harness already records that refusal.
