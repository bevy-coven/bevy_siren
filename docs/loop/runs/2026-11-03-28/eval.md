# eval.md — baseline and the `cues.yaml` table, iteration 28

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 112 + 5 doctests green |
| `cargo test --features serde` | 119 + 5 green |
| `cargo test --features mixer` | 120 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 % — watched). No open issues. Iteration 27's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. **No case parents a voice to an
entity**, so no case could see this.

## The lead, and what it became

Iteration 27's lead was `Play::on(entity)` and a despawned parent. Probed:

| path | result |
|---|---|
| Bevy's default path | the voice and its sink are gone; the log keeps `played` and no `stopped` |
| the crate's **mixer** | the voice entity is gone but **`stop_voice` is never called** — the player sounds for ever, slot held |

The second is rank 1: a **leak**, and a **two-backend divergence** the crate's own rule forbids. It is also invisible on
the default path, which is why 27 iterations did not see it.

## Ranked

1. **A despawned voice keeps its mixer slot for ever** — rank 1 (a leaked resource and a backend divergence). **The pick.**
2. **No `stopped` line for a parent's despawn on either backend** — a missing record; needs a new `Entry` reason, so the
   same public-enum decision five other items wait on.
3. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
4. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| parented loop voice, owner despawned (mixer) | `stop_voice [0]` once; slot freed; mix counts 0 | green (`[]` before) |
| owner alive for ten frames (guard, same test) | `stop_voice []` | green (unchanged) |
| every other test, both feature sets | unchanged | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
| the log after a despawn | still `played` and no `stopped` | **unchanged — the record gap, named** |
