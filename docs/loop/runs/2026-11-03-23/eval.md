# eval.md — baseline and the `cues.yaml` table, iteration 23

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 101 + 5 doctests green |
| `cargo test --features serde` | 108 + 5 green |
| `cargo test --features mixer` | 109 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 15 GB free. No open issues. Iteration 22's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (`state: red`, blocked on the gain decision); the rest green. Case 17 —
*"The same seed replays the same sounds … two seeds are two streams"* — is the case this iteration touches.

## The lead, and what it became

Iteration 22's lead was `Rng`'s period. Probed, **clean**: the 4-bin histogram over 400 000 draws is flat
(99754 / 99824 / 100469 / 99953), the 8-draw window did not repeat in 20 000 000 draws, and `next_f32` never reaches 1.0
(max 0.999999583 over 2 M draws from `u64::MAX`).

The probe that paid was the **seed**:

| | before |
|---|---|
| `Rng::new(0)` and `Rng::new(0x5EED_51AE_0000_0001)` | **identical streams** — the zero-state mapping |
| `SirenSettings::default_seed()` | **is** `0x5EED_51AE_0000_0001` |
| `seed: 0` through the plugin | `SirenChoices::seed()` reports **`0`**, for a stream built from `0x5EED…` |

So `seed: 0` and the default are one stream under two names, and the method documented as *"the seed this stream was
built from"* named the wrong one. Also found, and blocked: an empty `sound` key parses, is played, and is not reported
(`needs-decision/empty-sound-key.md`).

## Ranked

1. **`SirenChoices::seed()` reports a seed the stream did not come from** — the reproducibility promise, no API change
   needed. **The pick.**
2. `CueProblem` for an empty `sound` key — new decision file, blocked.
3. `CueProblem` for a duplicate key (`duplicate-sound-problem.md`, iteration 22).
4. `CueProblem::BadGain` (`bad-gain-problem.md`, iteration 18; case 31 red).
5. `Refusal::NotALoop` (iteration 16).

## After the change

| case | after | green? |
|---|---|---|
| `seed: 0` | `seed()` reports `0x5EED51AE00000001`, the value the stream started at | green (reported `0` before) |
| default seed / `seed: 1` (guards) | `0x5EED…` / `1` — unchanged | green |
| a stream 64 draws on | its `seed()` still names the same value | green — the test that caught my own first attempt |
| `seed: 0` and the default | one stream, pinned as behaviour | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
