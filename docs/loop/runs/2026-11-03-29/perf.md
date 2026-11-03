# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **3.1 / 5.2 / 6.6** — the quietest window this loop has recorded.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.057 | 0.0071 |
| 64 | 0.322 | 0.0050 |
| 256 | 1.342 | 0.0052 |

**Unchanged, and there is nothing to measure.** This iteration touches `README.md` and `docs/media/` only — **no `src/`
line moved**, so the crate's per-frame work is identical in source. The 256-voice figure (1.342 µs) sits inside the
0.62–1.34 µs spread this binary has shown across iterations 27–28 on a loaded machine.

## What this iteration did measure

The **panel's** own cost, on the quietest machine state yet: `siren: frames 140 mean_ms 9.245`, against 8.3–13.6 ms across
earlier captures. That is `examples/mixer`'s number — it draws every cue and ten log lines per frame — and is not the
crate's.

## The check (the real before/after)

`check_media.py`, run against the fixed tree and against a scratch reproduction of the pre-fix state:

| tree | result |
|---|---|
| fixed | 6 ok, **0 problems**, exit 0 |
| pre-fix reproduction | `ORPH docs/media/mixer-panel.gif is named in a README`, **1 problem**, exit 1 |
