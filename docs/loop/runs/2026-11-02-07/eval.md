# eval.md — baseline and the `cues.yaml` table, iteration 7

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 53 + 4 doctests green |
| `cargo test --features serde` | 60 + 4 green |
| `cargo test --features mixer` | 59 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 2.0 GB free, 100 % used. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 25 cases green. Cases 20–21 (an ended once-voice frees its slot) prove the **budget** is released; none proves the
**backend's own list** is. That is the gap.

## The gap, and the pick

A `MixerBackend` keeps `voices: Vec<VoiceTrack>` per bus and only ever appends. `retire_voices` despawns the Bevy
entity; the backend's slot stays, holding the `rodio::Player` and its queues. Probed on this Mac's real audio device:

```
400 one-shots, all samples ended:  live Bevy voices = 0,  slots held by the mixer = 400
```

An unbounded allocation per sound played, on the long-lived-mixer path — the backend whose whole point is that one
graph runs for the session. Iteration 2's log named it and deferred it; it was never picked up.

Ranked:

1. **The mixer never frees a voice slot** — rank 5's "costs little to run", and the cost that grows with session
   length. **The pick.**
2. A variant that distinguishes *undecodable* from *never mapped* — `needs-decision/undecodable-sound.md`; needs the
   owner.
3. A payload that names the cap — `needs-decision/refusal-names-its-cap.md`; needs the owner.
4. `Completes` can never be true — `needs-decision/remove-completes.md`; needs the owner.
5. A crossfade interrupted by a second change — a design question, not a defect.

Items 2–4 are decision-blocked; 5 is a feature. The pick is the only rank-1-class item that is buildable today.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a slot is reused** | 400 plays hold 6 slots, not 400 | green (`tests/mixer.rs`) |
| **a stale index addresses nothing** | `finished` says ended; gain/stop are no-ops on the new occupant | green (same test) |
| **an index stays a valid name** | each hand-out is a fresh generation; 200 plays, 200 distinct names | green (same test) |
| **the encoding round-trips** | slot 0 and the widest slot at generations 1 and `u32::MAX` | green (`tests/model.rs`) |
