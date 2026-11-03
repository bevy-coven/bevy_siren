# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **5.7–6.3** — busier than the runs above, which is why the spread is wide.

| voices | run 1 µs/f | run 2 µs/f | run 3 µs/f |
|---|---|---|---|
| 8 | 0.057 | 0.063 | — |
| 64 | 0.197 | 0.362 | — |
| 256 | **0.411** | 1.284 | — |

**The spread is the machine.** The same binary read 0.411–1.284 µs at 256 voices on two consecutive runs at that load; iterations
41–43 read the same envelope. `perfcheck` builds no `MusicScore`, so this iteration's accessor is not in its path.

**What the change costs.**

- `MusicState::blocked` is one linear scan over `layers` — two in the example, at most the score's length — returning a
  `Copy` `Option<usize>`. It allocates nothing.
- The panel calls it once per drawn layer, in the example only.
- `src/`'s per-frame work is unchanged: no system was added, and `start_missing`, `adopt_layers` and `move_layers` are
  untouched.

**The probe** — `evidence/panel/`, all read:

| | before | after |
|---|---|---|
| a layer the budget refused | `combat -80.0 dB` — same as a silent layer | `combat -80.0 dB **waiting**` |
| a layer the score lacks | — | `blocked` answers `None` |
| after one slot frees | — | `(calm: None, combat: Some(1))` |
| `SIREN_TIGHT` scope | table-wide, **starved the whole script** | the `Music` bus only |
| `cues.yaml` | 31 cases | **33** (32: parenting, 33: the virtual voice) |
| tests | 122 / 129 / 132 + 5 | unchanged — the assertions went into an existing test |
