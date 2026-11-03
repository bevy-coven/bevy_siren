# eval.md — iteration 44

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 122 + 5 |
| `cargo test --features serde` | green — 129 + 5 |
| `cargo test --features mixer` | green — 132 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **6.9 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md`; its recommendation defers the variant to the next minor, and the crate is 0.1.0 |
| 32 | **new** — a parented sound whose owner is despawned | green | `tests/play.rs::a_despawned_owner_stops_its_sound_and_says_so` |
| 33 | **new** — a layer the budget refused is a virtual voice | green | `tests/music.rs::a_refused_layer_start_is_tried_again_when_room_frees` |

## The item

Cases 32 and 33 are **new**, and both were red in the *panel* before their iterations fixed them: a parented sound's end
had no line (43), and a waiting layer drew the same `-80.0 dB` as a silent one (44). Case 33 is this iteration's own.

Probing for something larger found nothing: `stop_requests`, `room_for`, `VariantPicker::pick_from`, `PitchRange::at`,
`MusicState::live` and `reconcile` were each read and each already correct against its source.

## After

All green: 122 / 129 / 132 + 5, clippy, fmt, rustdoc, examples, `readme::`. `cues.yaml` has 33 unique ids. See
`review.md` for the three verdicts.
