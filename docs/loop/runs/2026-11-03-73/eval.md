# eval.md — iteration 73

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 45.2 s after a clean |
| `cargo test` | green — **198** + 6 |
| `cargo test --all-features` | green — **356** |

**Nothing red.** Working tree clean at `6f86607`. Case 31, the crate's only long-standing red case, went green in
iteration 72 and stayed green.

## Machine state

| | |
|---|---|
| start | **1.1 GB free**, `target` ~16 GB |
| after `cargo clean` | **16 GB free** (15.9 GiB removed) |
| when the all-features link failed | **323 MB** — `cc: error: linker command failed` |
| after a second clean | **15 GB free** (14.7 GiB removed) |

**Sixth disk failure in this project**, and the first one that presented as a *linker* error rather than a `rustc` I/O
error. Same cause, same fix.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged; case 31 green since iteration 72 |

**No case is red.** For the first time in this loop's recent history, the whole of `cues.yaml` is green.

## After

`bevy_siren`: **198 default / 367 all-features**, clippy 0, fmt, rustdoc — green.
