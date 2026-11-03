# eval.md — baseline and the `cues.yaml` table, iteration 14

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 73 + 4 doctests green |
| `cargo test --features serde` | 80 + 4 green |
| `cargo test --features mixer` | 80 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 12 GB free. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 30 cases green. Case 15 proves layers change on the bar; case 23 proves a layer takes its fade. **No case edits a
`MusicScore` at runtime**, which is the path its doc names.

## The gap, and the pick

Iteration 13 named the class that keeps paying: what a **public resource's** contract promises. `MusicScore` is one, and
probed, two edits break it in silence:

```
remove a layer:      names = ["combat"]        voices still live = 2
retarget a layer:    the score says track.b    the voice is track.a, and track.b never played
```

A removed layer leaves a **loop** voice with no owner; `retire_voices` skips loops by design, so it plays until the
process ends. A retargeted layer plays the wrong sample forever. Neither logs anything.

Ranked:

1. **A score edited at runtime leaves its voices behind** — the mix is not the mix the score describes. **The pick.**
2. The five decision files — public-enum questions for the owner.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 WAVs byte-identical; music still on the bar | green |
| **a removed layer stops its voice** | one voice left, `stopped … layer removed` | green (fails on the unfixed tree) |
| **a retargeted layer plays the new cue** | only the new cue is live, `stopped … layer retargeted` | green (fails on the unfixed tree) |
| **a gain edit keeps the voice** | same entity, same cue, no restart | green |
| **`names()` stays in score order** | `["calm", "combat"]` after a retarget | green |
