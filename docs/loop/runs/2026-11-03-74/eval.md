# eval.md — iteration 74

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — **198** + 6 |
| `cargo test --all-features` | green — **367** |

**Nothing red.** No case in `cues.yaml` is red. Working tree clean at `7ad8c54`.

## Machine state

| | |
|---|---|
| start | **8.7 GB free**, `target` 6.5 GB |
| before the render | **3.2 GB** — the release build spent ~5 GB |

No clean was needed this iteration; the previous iteration's clean plus a smaller target left room.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged, and case 31 green since iteration 72 |

## After

`bevy_siren`: **198 default / 367 all-features**, clippy 0, fmt, rustdoc — green. Plus **10 lib tests** for the mel path,
which are unit tests inside `src/semantic/mel.rs` and so are not in the integration count.

**The visual gate holds**: 32 render files, 14 distinct cue hashes, **−21.5 LUFS** integrated and **−7.5 dBFS** true peak
on the mix, the duck recovering by **1.97 s** — all identical to iteration 67, so nothing regressed.
