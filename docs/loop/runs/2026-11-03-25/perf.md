# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **9.6 / 9.2 / 10.4** — the steadiest window this loop has recorded.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.061 | 0.0076 |
| 64 | 0.348 | 0.0054 |
| 256 | 1.244 | 0.0049 |

**Unchanged, and there is nothing to measure.** This iteration touches `examples/mixer.rs` only: one `unwrap_or_else`
replacing `unwrap_or_default`, and one `is_empty` branch, both in `capture_config`, **once at startup**. No `src/` line
changed, so the crate's per-frame work is byte-identical in source and within 2–5 % here, which is the machine.

**The harness's own number, which is new:** the panel runs at **8.3–13.6 ms mean frame** across the captures
(`siren: frames 140 mean_ms 9.098`), i.e. it is a debug view, not a game loop — it draws every cue and 256 log lines each
frame. That figure is the example's, not the crate's, and is recorded because it is the first time this loop has measured
the visible harness at all.

## The probe (the real before/after)

`evidence/panel/default-run{,-before}.txt`:

| command | before | after |
|---|---|---|
| `SIREN_FRAMES=140`, `SIREN_KEYS` unset | `panicked … remainder with a divisor of zero`, **no still** | `siren: frames 140 mean_ms 9.098`, still written |
| `SIREN_KEYS=bogus` | panic | one line naming the typo, then the default script |
| `SIREN_KEYS=layer` | works | works, unchanged |
| no environment | panic | 20 s, no panic |
