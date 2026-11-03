# eval.md — baseline and the `cues.yaml` table, iteration 12

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 68 + 4 doctests green |
| `cargo test --features serde` | 75 + 4 green |
| `cargo test --features mixer` | 75 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 14 GB free. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 29 cases green. Case 23 (`a_cue_can_be_stopped`) stops a **live** loop; no case stops a voice that has **ended**.

## The gap, and the pick

Iteration 11's log advised picking a new case that turns something red. Probed, and one did:

```
play a one-shot -> hold Played::entity -> the voice ends and retire_voices despawns it
-> StopVoice(that handle)
   panicked at bevy_ecs-0.20.1/src/error/handler.rs:128
   Entity despawned: The entity with ID 43v0 is invalid; its index now has generation 1.
```

`stop_requests` took the entity straight from a message and commanded it. Every other `commands.entity(..)` in the crate
takes an entity from a live query, which cannot be dead; this was the one path where the id came from outside — and
`Played::entity` is documented as the handle to keep *for `StopVoice`*. The crate's own `retire_voices` is what makes it
stale.

Ranked:

1. **A stale `StopVoice` panics the game** — a crash, from the crate's documented workflow, on the crate's own
   retirement path. **The pick.**
2. The five decision files — public-enum questions for the owner.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a stop for an ended voice is a no-op** | updates cleanly, nothing logged | green (the unfixed tree **panics** inside `app.update()`) |
| **a second stop for a fading voice is skipped** | one `Stopped` line, one fade | green |
| **`StopCue` still stops every voice of a cue** | both voices stopped | green |
