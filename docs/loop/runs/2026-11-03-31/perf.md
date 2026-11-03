# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.9 / 3.7 / 5.1** — quiet.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.066 | 0.0082 |
| 64 | 0.332 | 0.0052 |
| 256 | 0.884 | 0.0035 |

**Unchanged.** This iteration touches `examples/mixer.rs` only — one match arm's annotation — and `tests/mix.rs`. No `src/`
line moved, so the crate's per-frame work is identical in source; the 256-voice figure sits inside the 0.62–1.34 µs spread
this binary has shown across iterations 27–30.

**What the annotation costs, where it runs.** The panel scans `log.recent(SirenLog::CAPACITY)` — 256 entries — **once per
bus that is currently ducked**, and only for the four bus rows. With two ducks active that is at most 512 `Entry` matches
per frame on a debug view. It is bounded by `CAPACITY`, not by history.

## The probe (the real before/after)

`evidence/panel/`, and a direct probe of the released state:

| | result |
|---|---|
| ducked, 1280×720 | `Music  -9.5 dB  duck -8.0 dB **by dialogue.greeting**  2 / 12 voices` — one line, no wrap |
| ducked, 1920×1080 | same annotation, more room |
| the log's own claim, same picture | `1.57s dialogue.greeting ducked Music by -8.0 dB` |
| **after the duck releases** | `duck +0.00 -> row shows no duck` — the clause is inside the `duck < -0.05` branch |
| a bus nothing ducked | no name (`by("Ambience") == None`) |
