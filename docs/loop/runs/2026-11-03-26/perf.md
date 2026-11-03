# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **7.4 / 8.9 / 9.6**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.059 | 0.0074 |
| 64 | 0.340 | 0.0053 |
| 256 | 1.219 | 0.0048 |

**Unchanged.** This iteration touches `examples/mixer.rs` only — one `Kind` variant set, one match arm, one helper, one
env knob — so no `src/` line moved and the crate's per-frame work is identical in source. The rows are within 2 % of
iteration 25's.

**The panel's own cost, which is the number that changed:** `problems_in` is called **once per frame** by `draw_panel` (twice
in the fault case: once for the block, once for the row markers), and it walks the cue list — eleven cues, trivial. The
panel's frame time is **8.3–9.7 ms** across the captures, against iteration 25's 8.3–13.6; the spread is the machine, and
the block is not measurable in it. That figure is the example's, not the crate's.

## The probe (the real before/after)

`evidence/panel/`:

| table | before | after |
|---|---|---|
| clean (11 cues, `Ambience` + `Music` listed) | no `Problems` section | `Problems` / `none`, no row marked |
| `SIREN_FAULTS=1` (+ a cue on `Reverb`, + an inverted pitch) | the two bad cues drawn as healthy | both rows end `<- problem`; two sentences in the block |
| `problems()` instead of `problems_in` | — | would print **five false `UnknownBus`** on the clean table |
