# Iteration 76 — perf

The change is in `src/semantic`, behind a non-default feature, and touches no audio path. `perfcheck` measures the mix
engine (`update_mix` at 8, 64 and 256 voices), which this iteration does not touch.

| Case | Before | After |
|---|---|---|
| 19 — frame cost at 8 / 64 / 256 voices | — | **not re-measured** |

**Why not, stated rather than glossed.** Measuring it needs `cargo run --release --example perfcheck`, and a release
build wants roughly 2 GB. The volume was at **100 %, 122 MB free** at the start of this iteration and is at **586 MB**
now; a release profile build of the `bevy` umbrella plus its examples does not fit, and attempting it would fail as
`ld: write() failed, errno=28`, the same non-code failure that cost six baseline steps earlier in this run.

**Why it cannot have moved.** Case 19's subject is `update_mix`, and:

- `git diff --name-only` over this iteration lists **no** file outside `src/semantic/` and `docs/` — verified, count 0.
- `semantic` is not in `default = []`, and `src/semantic` is behind `#[cfg(feature = "semantic")]`, so the binary
  `perfcheck` builds is compiled from **byte-identical** sources to iteration 75's.
- Iteration 75 recorded the same figure as 67, 71 and 74 (−21.5 LUFS, −7.5 dBFS, 14 distinct cues); this iteration
  changes nothing that could move it.

The number is owed, not waived: **the next iteration should run `perfcheck` once disk allows**, and until it does, case
19 is inherited from iteration 75 rather than re-asserted here.
