# Iteration 81 — perf

`cargo run --release --example perfcheck`, Apple M4, `--release`, idle machine.

| voices | µs/frame | µs/voice | vs iteration 80 |
|---|---|---|---|
| 8 | **0.064** | 0.0080 | 0.084 → 0.064 (−24 %) |
| 64 | **0.355** | 0.0055 | 0.472 → 0.355 (−25 %) |
| 256 | **1.341** | 0.0052 | 1.576 → 1.341 (−15 %) |

**Faster, and not because of this change.** Nothing in the audio path moved — `git diff` touches `src/semantic/` only,
which the default build does not compile. The difference is run-to-run variance on a shared machine, which is why the
skill says a shared machine's load belongs in the numbers: three measurements of identical code now read
0.064–0.084 / 0.355–0.472 / 1.341–1.576 µs. **The honest reading is "no regression", not "an improvement."**

Per-voice cost still falls with count (0.0080 → 0.0052), which is what a fixed per-frame cost amortised must do.
