# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.3–3.8**.

| voices | run 1 µs/f | run 2 µs/f | run 3 µs/f |
|---|---|---|---|
| 8 | 0.062 | 0.019 | 0.056 |
| 64 | 0.358 | 0.102 | 0.339 |
| 256 | 1.241 | **0.402** | 1.360 |

**The spread is the machine, not the commit.** The same binary read 0.402–1.360 µs at 256 voices on three consecutive
runs; iteration 41 read 0.416–1.392 with no `src/` change at all. `perfcheck` does not run `examples/mixer` and does not
call `Entry::line`.

**What this iteration costs.**

- `room_for` returns `Vec<(Entity, Cap)>` instead of `Vec<Entity>`. `Entity` is 8 bytes and `Cap` is 1, so a stolen
  voice is 12 bytes against 8 — and `stolen` is empty on the overwhelming majority of plays. The three filters that used
  `stop.contains(&v.entity)` now use `stop.iter().any(|(e, _)| *e == e)`; a play steals at most a handful, so this is a
  comparison over ≤ 3 items where the old code was one.
- `Cap::as_str()` is a `match` over three unit variants, called once per **stolen** voice.
- The row's `format!` gained one argument. The panel runs in the example.

**No case is slower**: `update_mix`, `retire_voices`, `move_layers` and `start_pending` are untouched.

**The probe** — `evidence/panel/`, all read:

| | before | after |
|---|---|---|
| how many of a cue may sound | **not shown** | `x2` / `x1` / `all` beside `priority` |
| `max_instances: 0` | not shown | `all`, paired with `UnlimitedInstances` in `Problems` |
| a cue-cap steal | `stolen by footstep.grass` | `stolen by footstep.grass **(cue)**` |
| a bus-cap steal | `stolen by sword.hit` | `stolen by sword.hit **(bus)**` |
| three requests at `x2` | — | 2 voices, **1** steal (isolated run) |
| the default script | 6 steps | 7 — `poly` last, so the still keeps the duck/stinger/spatial log |
| the still's margin | 9 px | 9 px |
| tests | 120 / 127 / 129 + 5 | **121 / 128 / 130 + 5** |
