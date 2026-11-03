# eval.md — baseline and the `cues.yaml` table, iteration 13

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 71 + 4 doctests green |
| `cargo test --features serde` | 78 + 4 green |
| `cargo test --features mixer` | 78 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 12 GB free. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 30 cases green. Case 7 (`a_cue_respects_its_own_polyphony`) asserts the cap **at play time**; no case edits the
table under a live voice, so it cannot see the difference between "a budget on plays" and "a standing count".

## The hunt, and what it found

Iteration 12's log named the hunt: hold a public handle across a frame and use it as documented. This pass ran it over
every remaining carrier — `Play::on(entity)` in both despawn orders, `MusicState::voice`, `SirenChoices`, the `Samples`
handle — and all were safe. The audit of every `commands.entity(..)` confirms iteration 12 closed the only unguarded
path. The one thing the hunt found was not a crash but a **documented promise with two readings**: `max_instances`
says "may sound at once", and a table edited under live voices leaves two sounding against a cap of one.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 WAVs byte-identical; music still on the bar | green |
| **a cap lowered under live voices** | next play refused; both voices still sounding; nothing stopped or logged | green (`a_cap_lowered_under_live_voices_does_not_end_them`) |
| **a cap raised under live voices** | the next play is admitted | green (`a_cap_raised_under_live_voices_lets_the_next_play_through`) |
| **the rejected design** | removing `room_for`'s unstealable guard reddens the lowered-cap test | measured |
