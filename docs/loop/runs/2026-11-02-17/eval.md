# eval.md — baseline and the `cues.yaml` table, iteration 17

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 83 + 4 doctests green |
| `cargo test --features serde` | 90 + 4 green |
| `cargo test --features mixer` | 90 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 17 GB free. No open issues. Iteration 16's run has its `review.md`.

## The `cues.yaml` table

All 30 cases green. Cases 20 and 21 exercise the budget's **release** path; **nothing exercises a voice that can never
release**, because a voice with no player is not a state any case constructs.

## The hunt, and what it found

Iteration 16 left one lead: the **two-backend** promise. Probed:

| probe | result |
|---|---|
| a **steal** on both paths | identical `SirenLog` stream, field for field |
| a scripted **duck**, `bus_db` / `ducked`, 90 frames | **identical** on both paths; min duck −8.000 dB on each |
| `Backend(true)` with **no `Output`** | **rank 1**: the voice has no `AudioPlayer` and no mixer |

The steal and duck rows are clean — the two-backend claim holds. The third is not about the backends answering
differently; it is about a backend answering **nothing**.

## Ranked

1. **`Backend(true)` with no `Output` gives a voice nobody plays** — silent, unretirable, holds its slot forever.
   **The pick.**
2. `Refusal::NotALoop` (iteration 16's item 1; decision file written, waiting).
3. Re-time a restarted stem to the bar (iteration 15's item 1).
4. Quantise a retarget to the bar (iteration 14's item 1).
5. A `Refusal` payload that names the cap.

## After the change

| case | after | green? |
|---|---|---|
| `Backend(true)`, no `Output`, one play | the voice carries an `AudioPlayer`; `Played` is written | green (fails before) |
| `Backend(true)` **with** an `Output` | the voice has **no** `AudioPlayer` — the mixer owns it | green before and after (the guard) |
| a steal, both paths | unchanged | green |
| a duck, both paths | unchanged | green |
| every rendered cue | unchanged | green |
