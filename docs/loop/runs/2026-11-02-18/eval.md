# eval.md — baseline and the `cues.yaml` table, iteration 18

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 84 + 4 doctests green |
| `cargo test --features serde` | 91 + 4 green |
| `cargo test --features mixer` | 92 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 17 GB free. No open issues. Iteration 17's run has its `review.md`.

## The `cues.yaml` table

All 30 cases green. Case 18 exercises `problems()` on four mistakes; **none of them is a gain that is not a number**,
and no case authors one.

## The hunt, and what it found

Iteration 17's lead was the `AudioOutput` trait's contract. Probing that found `Applied` records what the crate
**decided**, not what the backend took — and a backend that ignores `set_voice_db` (as the test `FakeOutput` does) cannot
desynchronise anything the crate reads, so the lead was **clean**.

The probe that paid was a different one: what a caller value that is not a number does.

| probe | result |
|---|---|
| a cue authored `gain_db: NaN` | **rank 1**: `problems()` silent, `Played gain -80.00`, the sound **silent** |
| `Play::gain(NaN)` / `NaN` on a slot | same |
| `Play::pitch(NaN)` | `Played.pitch` reports `NaN` for a sound at `1.0×` |
| `Play::pitch(1e9)` | reports `1e9` for a sound at `16×` |
| a non-finite **distance** | **already** the neutral `0.0` m, documented — the crate's own policy |
| `PitchRange` non-finite | already guarded and reported (`InvertedPitch`) |

So the crate had **two answers** to one caller mistake, and the documented one is the neutral value.

## Ranked

1. **A gain that is not a number is silent, and the meter says it is a level** — rank 1: silent wrong output, a missing
   report, and `Played` disagreeing with the device. **The pick.**
2. `Refusal::NotALoop` (iteration 16's item 1; decision file waiting).
3. A `Refusal` variant for a bad gain or detune — new `needs-decision/bad-gain-problem.md`.
4. Re-time a restarted stem to the bar (iteration 15's item 1).
5. Quantise a retarget to the bar (iteration 14's item 1).

## After the change

| case | after | green? |
|---|---|---|
| a cue authored `gain_db: NaN` | `gain 0.00`, audible, the level it authored | green (fails before) |
| `Play::gain(NaN)` and `gain(-∞)` | `gain 0.00` | green (fails before) |
| `Play::pitch(NaN)` | `pitch 0.00` — the `1.0×` that plays | green (fails before) |
| `Play::pitch(1e9)` | `pitch 48.00` — the clamp that plays | green (fails before) |
| a bus at `SILENT_DB` | still silent | green before and after (the guard) |
| every other case, every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
