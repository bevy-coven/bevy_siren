# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.2–3.9**.

| voices | run 1 µs/f | run 2 µs/f | run 3 µs/f |
|---|---|---|---|
| 8 | 0.061 | 0.021 | 0.055 |
| 64 | 0.352 | 0.104 | 0.331 |
| 256 | 1.220 | **0.408** | 1.310 |

**The spread is the machine.** The same binary read 0.408–1.310 µs at 256 voices over three runs; iterations 41 and 42
read the same envelope with no `src/` change at all. `perfcheck` does not despawn parented voices, so this iteration's
system is not in its path.

**What the system costs, where it runs.** `log_despawned_voices` runs once per frame and:

- iterates every voice once, inserting into a `BTreeMap` — **O(live log n)**, 4 voices in the example, at most
  `max_voices` (32 by default) in a game;
- reads `RemovedComponents<Voice>`, which is a message reader over an almost-always-empty queue;
- allocates nothing when nothing was removed, and one `String` clone for the reason only when a line is written.

The `retain` at the end is one pass over the same map. **No per-frame allocation** on a frame where no voice vanishes,
which is every frame of `perfcheck`.

**The probe** — `evidence/panel/`, read:

| | before | after |
|---|---|---|
| a parented sound's end | **no line** on either backend | `0.68s stopped \`engine.idle\`: owner despawned` |
| the mixer's own line | slot freed, **silent** | one line, written by the shared system |
| a game's own unparented despawn | silent | **still silent** (`ChildOf` gates it) |
| a double line | — | caught by test, removed |
| the panel | no `parent` step | `parent` in the default script; the log and `Effects 1 / 2 voices` agree |
| tests | 120 / 127 / 129 + 5 | **122 / 129 / 132 + 5** (two new) |
