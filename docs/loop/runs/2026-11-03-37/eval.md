# eval.md — baseline and the `cues.yaml` table, iteration 37

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 119 + 5 doctests green |
| `cargo test --features serde` | 126 + 5 green |
| `cargo test --features mixer` | 128 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 11 GB free (98 % — watched; a release build wants ~2 GB). Iteration 36's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. **No case captures a GIF**, and none
could: the property is temporal.

## The lead, and what it became

Iteration 36's log left the check *"does the guide's 'layers that change on the bar' now show end to end in the GIF?"*
Probed with arithmetic first, then with the capture directory:

| measurement | value |
|---|---|
| `MIX_TAU_S` (a layer fade) | **50 ms** |
| the example's frame time | **~8.3 ms/update** |
| frames a fade lasts | **~6** |
| the shipped GIF's sampling | every **12 updates** |
| **samples per fade** | **0.5** |

The shipped GIF was **right by luck**: 20 frames, with the music line's ink changing only across frames 12–16.

Two probes came back clean on the way: a move to the gain a layer already has is a no-op (`moving 0`), and a 2.4 s fade
arrives in 161 frames against the 144 a 60 fps clock predicts — `tau_for_fade`'s documented `fade/ln(1000)`.

## Ranked

1. **The GIF samples a fade by updates, not by time — 0.5 samples per fade** — the last iteration's feature appears by luck.
   **The pick.**
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. A gain above 0 dB (`needs-decision/above-unity-gain.md`; recommendation to not add it).

## After the change

| case | after | green? |
|---|---|---|
| `SIREN_SHOT_EVERY` | **hundredths of a second** (4 = 40 ms) | green (it was an update count) |
| samples per 50 ms fade at `EVERY=4` | **1.25** | green (0.5 before) |
| frames carrying the arrow | **11 consecutive** (`f_0028`–`f_0038`) | green (1, by luck) |
| the GIF | **46 frames at 10 fps** | green (20 at 6) |
| the crop, read | `calm -76.2 -> -80.0   combat -6.9 -> -3.0` | green |
| every test, both feature sets | unchanged (119 / 126 / 128 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
