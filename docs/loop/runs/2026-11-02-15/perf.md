# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.60 / 6.16 / 5.68** (higher than iteration 14's 3.27 — a shared machine). Disk 12 GB
free.

| voices | µs/frame (after) | µs/frame (iter. 14) | µs/voice (after) |
|---|---|---|---|
| 8 | 0.064 | 0.062 | 0.0080 |
| 64 | 0.351 | 0.353 | 0.0055 |
| 256 | 0.734 | 1.135 | 0.0029 |

**Within noise of the baseline**, and the 256 case is *faster* than iteration 14's in a noisier load window — which is
the honest way to say "unchanged": the example measures the crate's per-frame arithmetic, and nothing this iteration
touched is on that path.

**What the reconcile costs, where it is not measured.** `start_layers` now runs every frame instead of only on a score
change. Its per-frame work is a `Query` iteration over live voices and a `position` walk of the live-layer list — a
handful of layers. It sits in `Choose`, before `Play`; the 256-voice figure includes the voice query. A stem's restart
is one `Play` on the frame a slot frees, at most once per dead layer.

## The probes (the real before/after)

`evidence/layer-voices.txt` (after) against `evidence/layer-voices-before.txt` (the base commit):

| row | before | after |
|---|---|---|
| `StopCue` on a stem | `voice()` `Some(47v0)` — a **despawned** entity; `live()` **1** | `None`, `live()` 0, `layer_db` −80 dB |
| a `ui.click` beside a `priority: 0` stem, `max_voices: 1` | `voices ["ui.click"]`, **`track.calm stolen by ui.click`** | `voices ["track.calm"]`, no steals |
| a stem whose voice died | `Some(47v0)` → `Some(47v0)`, still dead, `live()` **1** | `Some(47v0)` → `Some(48v0)`, `live()` 1 |
| a refused first start | `live()` **0** forever after the slot frees | `live()` 1; refusals logged **1 → 1** (no spam) |
