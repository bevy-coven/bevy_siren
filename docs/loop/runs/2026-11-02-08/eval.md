# eval.md — baseline and the `cues.yaml` table, iteration 8

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 54 + 4 doctests green |
| `cargo test --features serde` | 61 + 4 green |
| `cargo test --features mixer` | 61 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 893 MB free, 100 % used. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 26 cases green. Case 14 (the distance law) proves a **spatial** cue at two distances; case 13 (no listener) proves
the refusal. No case covers the fourth combination — a **non-spatial** cue given a position.

## The gap, and the pick

Probed: `Cue::spatial = false` played with `.at((0,0,8))`.

```
spatial cue, no position:   gain_db = 0.00   (documented: "heard by everyone at the cue's own gain")
non-spatial cue, 8 m away:  gain_db = 0.00   (but the entity is routed through the two-ear panner)
```

The second row is the bug: `spawn_voice` set `PlaybackSettings.spatial` from `request.position.is_some()` **alone**, so a
cue that says it is not placed went through `rodio`'s panner, whose own law is `min(1, 1/d²)` —
**0.0117 linear, −38.7 dB at 8 m**. `Played.gain_db` said 0.00. The meter is what a game's level readout believes, so
nothing downstream could notice.

Ranked:

1. **A non-spatial cue given a position is panned at a law nobody wrote, while `Played` reports 0 dB** — rank 1, wrong
   output, silent, and the crate's own report is the lie. **The pick.**
2. An undecodable variant / a cap in the refusal payload / removing `Completes` — decision-blocked.
3. A crossfade interrupted by a second change — a design question.

Items 2 are the owner's call; item 3 is a feature. The pick is the only rank-1-class item buildable today.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **a non-spatial cue ignores a position** | flat at its own gain, no `Transform`, `Played.gain_db` true | green (fails on the unfixed tree) |
| **a spatial cue at a position is still panned** | −6.02 dB at 8 m, `Compensation` non-zero, `Transform` at the place | green |
| **a spatial cue without a position is flat** | unchanged, documented | green |
| **a spatial cue with no listener is refused** | `SpatiallyMute` | green |
| **the panner's own law** | 0.0117 linear / −38.7 dB at 8 m, pinned in `model` | green |
