# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **9.4 / 8.4 / 6.6**.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.123 | 0.0154 |
| 64 | 0.452 | 0.0071 |
| 256 | 1.235 | 0.0048 |

**Unchanged, and there is nothing to measure.** This iteration changes **doc comments** and adds a **test** — no executable
line moved. The 256-voice figure (1.235 µs) sits inside the 0.5–1.4 µs spread this binary has shown across iterations 33–34 on
a machine whose load average moved from 3.3 to 9.4 over the same span.

## The check (the real artefact)

`evidence/nine-checks.txt` — one score carrying every fault, printing the report's nine sentences. That file is what the doc
now promises, and what the test asserts.

| | before | after |
|---|---|---|
| sentences the report can produce | 9 | 9 (unchanged) |
| sentences the doc named | **4** | **9** |
| `RUSTDOCFLAGS="-D warnings"` | green | green — after fixing a redundant intra-doc link my first version introduced |
| test count | 118 / 125 / 127 + 5 | 119 / 126 / 128 + 5 |
