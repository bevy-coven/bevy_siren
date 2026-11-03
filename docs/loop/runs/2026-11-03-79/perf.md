# Iteration 79 — perf

No measurement, and nothing to measure: this iteration changes no executable code. Case 19 remains inherited from
iteration 75 and is still **owed** — `perfcheck` needs a release build, and the volume has had under 500 MB free for
five consecutive iterations.

The one latency this iteration *did* measure is the ADR's own subject and is recorded in `research.md`: a brute-force
`f32` dot over 5 000 × 512 embeddings takes **0.05–1.87 ms**, which is why the ADR declines an ANN.
