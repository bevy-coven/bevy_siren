# perf.md — frame cost at 8, 64 and 256 voices

**Machine state**: not re-measured this iteration; the previous run (iteration 15) was load 8.60 / 6.16 / 5.68 on this
shared Mac, and the code path is unchanged.

| voices | µs/frame (iter. 15) | µs/voice |
|---|---|---|
| 8 | 0.064 | 0.0080 |
| 64 | 0.351 | 0.0055 |
| 256 | 0.734 | 0.0029 |

**Why there is no new number.** The change adds one `CueTable` lookup per layer that has **no voice** — a handful of
layers, and only on a frame a layer is unstarted. It adds no per-voice work, so the 8/64/256 figures (which cost each
voice) cannot move. The measurement that matters is the play count, below.

## The probes (the real before/after)

`evidence/one-shot-layer.txt`:

| | before | after |
|---|---|---|
| plays in 10 frames, a `Mode::Once` layer | **10** | **0** |
| `Refused` count | 0 | 0 (still a log line, not a message — see the decision file) |
| log | `played hit on Music at -80.0 dB` | `refused hit: music layer calm names cue hit, which is not a loop, so it cannot hold a layer` |
| an unknown cue (for contrast) | `Refused` 1, one line | unchanged |
