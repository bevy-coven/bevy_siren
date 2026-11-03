# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **8.6 / 10.9 / 16.5** and drifting; the 15-minute figure sat near 16 through every run.
Five consecutive runs of the same binary:

| run | 64 µs/frame | 256 µs/frame |
|---|---|---|
| 1 | 0.252 | 0.393 |
| 2 | 0.133 | 0.597 |
| 3 | 0.394 | 1.305 |
| 4 | 0.358 | 1.341 |
| 5 | 0.386 | 1.350 |

**The spread is the measurement, not the code.** 256 voices ranged 0.393 → 1.350 µs across five runs of one binary on an
unchanged workload. Reporting a single point here would be reporting the scheduler. **Median 256 ≈ 1.31 µs**, against
iteration 21's 1.021 — within the spread, and **the change does not run on this path**: `pick_from` is called **once per
play**, not per frame per voice, and `update_mix`'s loop is untouched.

**What the change does cost, exactly:** one `Vec<&str>` of the cue's slot keys, built per play. A cue has a handful of
slots; a footstep at 20 Hz costs 20 of them a second. Not measurable here, and said so rather than claimed as zero.

## The probe (the real before/after)

`evidence/duplicate-keys{,-before}.txt`, 16 plays each, counting **adjacent equal sounds**:

| cue, `AvoidRepeat { avoid: 1 }` | before | after |
|---|---|---|
| `["a", "a", "b"]` | **3** adjacent repeats (`… b a a a b`) | **0** |
| `["a", "a", "a", "b"]` | **8** adjacent repeats (`a a a a b a a a`) | **0** |
| `["a", "b"]` (control) | 0 | 0 |
| `["a"]` (one sound, guard) | 15 | 15 |
