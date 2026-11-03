# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **7.9 / 12.7 / 16.3** — loaded, and falling through the run.

| voices | µs/frame (after) | µs/frame (iter. 20) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.055 | 0.046 | 0.0069 |
| 64 | 0.334 | 0.327 | 0.0052 |
| 256 | 1.021 | 0.991 | 0.0040 |

**Within 3–20 % in a window whose load average spanned 7.9 to 16.3.** The example costs the per-voice mix loop, and
**nothing in this diff runs there**: `problems_in` is a **host** call, made when the panel draws or a host checks a
table. The spread is the machine, and it is reported as measured rather than smoothed.

## The probe (the real before/after)

`evidence/custom-buses.txt`:

| call, with a `Sfx` bus in `SirenSettings::buses` | result |
|---|---|
| `settings.bus_names()` | `["Sfx", "Voices"]` — the cue is legal |
| `table.problems()` (**before**, and what the panel called) | `cue \`gunshot\` plays on bus \`Sfx\`, which the mix table does not have` |
| `table.problems_in(&settings)` (**after**) | `[]` |

`evidence/gain-report.txt`, the blocked half, unchanged and recorded as `cues.yaml` case 31 (`state: red`):

| field | reported? |
|---|---|
| `Cue::pitch` inverted | yes |
| `Cue::attenuation` negative | yes |
| `Cue::gain_db: NaN` | **no** |
| `SoundSlot::gain_db: NaN` | **no** |
