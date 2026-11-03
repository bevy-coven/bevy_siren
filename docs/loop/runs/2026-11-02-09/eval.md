# eval.md — baseline and the `cues.yaml` table, iteration 9

## Baseline, as recorded before any change (2026-11-02, this Mac, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 59 + 4 doctests green |
| `cargo test --features serde` | 66 + 4 green |
| `cargo test --features mixer` | 66 + 4 green |
| `cargo test --doc` | 4 green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |

**Nothing red.** Disk 828 MB free, 100 % used. Open issues: none.

## The `cues.yaml` table against the crate as it stands

All 27 cases green. Case 16 proves a stinger **ducks**; no case asks what happens when it **cannot play**.

## The gap, and the pick

Probed, three statements about one request:

```
Music ducked:        -9.00 dB
refusals from Play:  [MissingSound { sound: "stinger_01" }]
log:                 0.03s stinger ducked Music by -9.0 dB
log:                 0.03s refused `music.stinger-no-handle`: no handle for sound `stinger_01`
```

`play_stingers` writes the `Play` and arms the duck in the same breath; `Play` is answered later in the frame. A refused
stinger therefore dips the music for a sound nobody hears, and the log states both facts on adjacent lines. The same
bug reaches a cue **not in the table** through `unwrap_or("Music")`, which let it pass the bus check.

Ranked:

1. **A stinger that cannot play still ducks the music, and the log contradicts itself** — rank 1: a duck that
   corresponds to no sound. **The pick.**
2. A log line for an ignored position, an undecodable variant, a cap in the refusal payload, removing `Completes` — all
   decision-blocked.
3. A crossfade interrupted by a second change — a design question.

## After the change

| Case | After | Green? |
|---|---|---|
| everything above | unchanged; 16/16 cue/mix/voices WAVs byte-identical | green |
| **an unknown cue ducks nothing** | Music 0.00 dB, `UnknownCue` as a message, one log line | green (fails on the unfixed tree) |
| **a missing sound ducks nothing** | Music 0.00 dB, only the refusal line | green (fails on the unfixed tree) |
| **a real stinger ducks on the frame it fires** | −9.0 dB, one `Ducked`, same frame | green |
| **a wrong-bus stinger is refused, no duck** | prose line in the log, music untouched | green |
