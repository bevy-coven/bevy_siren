# Iteration 116 — perf

Three costs, because `examples/perfcheck` measures a fourth. Release builds, this Mac (M4 Pro). Every number is in `evidence/after/`.

## 1. `perfcheck` — the model arithmetic, which this change does not touch

`perfcheck` costs `model::chain_db`, `model::db_to_gain` and `model::approach` per voice and per bus — **not the systems** (its header says so) — so it cannot see a new system. Three runs (`evidence/after/` has the render; the
output is in the transcript), µs a frame:

| voices | run 1 | run 2 | run 3 | iteration 115's runs | iteration 114's band |
|---|---|---|---|---|---|
| 8 | 0.064 | 0.062 | 0.059 | 0.057, 0.039, 0.057 | 0.018 – 0.074 |
| 64 | 0.355 | 0.355 | 0.336 | 0.333, 0.237, 0.266 | 0.106 – 0.395 |
| 256 | 1.217 | 1.189 | 1.245 | 1.220, 0.887, 0.411 | 0.396 – 1.339 |

Inside the band at every size; the run-to-run spread at 256 voices has been 3× on this machine (0.411 – 1.220 in iteration 115), so the high end is the machine's state and not this change, which this program cannot run. **What this says**:
nothing else moved.

## 2. `refuse_failed_loads` — a scratch bench in release, removed (`evidence/after/refuse-failed-loads-bench.rs.txt`, `refuse-failed-loads-cost.txt`)

µs a frame, per number of voices, by what the voices are:

| voices | running (a mixer voice each) | waiting, default handle, no asset | waiting, loaded asset |
|---|---|---|---|
| 0 | 0.058 | 0.055 | 0.098 |
| 8 | 0.054 | 0.110 | 0.077 |
| 64 | 0.066 | 0.247 | 0.149 |
| 256 | **0.059** | **0.769** | 0.417 |

A **running** voice costs nothing: its archetype (it carries a `MixerVoice` or a sink) is skipped by the query, so the system's whole price at 256 live voices is its fixed 0.06 µs. The **worst case** is a world in which no voice ever starts — a headless
app, or a game with no audio device — 256 voices waiting for ever: 0.77 µs a frame, 3 ns a voice (an `Assets::get`, then `get_load_state`, which answers `None` at once for a default handle). Prediction P5 was ≤ 0.1 µs with nothing waiting
and ≤ 10 µs at 256 waiting: right, and the second loose by an order of magnitude.

## 3. The check at a play — `load_error`, per call (`refuse-failed-loads-cost.txt`)

| handle | ns a call |
|---|---|
| a default (Uuid) handle | 2.2 |
| a handle the server tracks as **loaded** (the real loader, a real file) — the ordinary play | 8.5 |
| a handle the server tracks as **failed** (formats its error) | 137.8 |

P5 was ≤ 0.5 µs a play: right by 50×. **Measured on the first cut.** The version that shipped asks `sounds::cannot_load`, which looks in `Assets` first and returns for a loaded handle **before** it asks the server, so the ordinary play now does
strictly less than the 8.5 ns above; the failed path is the same two lookups and the same string. It was not re-benched, and the number above is the upper bound.

## Not measured

The cost of `refuse_failed_loads` in a game with a real device and real sinks (by construction 0.06 µs: every started voice is skipped); the asset server's lock under contention with a loader thread; and `play_requests`'s own per-play
cost with the extra parameter (one `Option<Res>` pair fetched once per frame, not per play).
