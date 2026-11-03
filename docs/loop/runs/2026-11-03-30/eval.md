# eval.md — baseline and the `cues.yaml` table, iteration 30

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 112 + 5 doctests green |
| `cargo test --features serde` | 119 + 5 green |
| `cargo test --features mixer` | 121 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 %). The `gh issue list` query timed out (TLS handshake — network, not the repo).
Iteration 29's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 18 has four simultaneous
mistakes and runs `problems_with` — **the case exists, but nothing drew a panel with them.**

## The lead, and what it became

Iteration 29's lead was `master_db` at `SILENT_DB`. Probed, **clean**: every bus reads `SILENT_DB`, no duck, no spurious
problem — `chain_db` propagates master silence, as documented. A second probe, **one answer per play**, also clean:
6 asks (one of every outcome) → 2 `Played` + 4 `Refused` = 6, each refusal naming the right thing.

The probe that paid was on the **panel**, and it came from looking at a capture:

| faults | before |
|---|---|
| six (`SIREN_FAULTS=many`) | the `Problems` block pushed the **`Log` heading and its ten lines off the bottom** of a 1280×720 capture |

The panel is a fixed column with no scroll, the block was unbounded, and the log is last — so the view lost the section that
answers *"why is the music quiet"* exactly when there was most to read.

## Ranked

1. **The panel's `Problems` block hides the log when there is most to read** — rank 1 for the panel; provable by looking.
   **The pick.**
2. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep now).
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| faults | after | green? |
|---|---|---|
| six | three sentences, `and 3 more; the log below is what played`, **log on screen** | green (log off screen before) |
| two (the README GIF) | both sentences, no count, log present | green (unchanged — bound does not fire) |
| none | `Problems` / `none` | green (guard) |
| every test, both feature sets | unchanged (112 / 119 / 121 + 5) | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
