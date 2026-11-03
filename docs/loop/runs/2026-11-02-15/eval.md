# eval.md — baseline and the `cues.yaml` table, iteration 15

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 76 + 4 doctests green |
| `cargo test --features serde` | 83 + 4 green |
| `cargo test --features mixer` | 83 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 12 GB free. No open issues. Iteration 14's run directory has its `review.md`.

## The `cues.yaml` table

All 30 cases green. Cases 15, 23 and 28 drive the score and a stinger; **none stops a layer's voice or edits a cue's
priority**.

## The gap, probed

| probe | observation | verdict |
|---|---|---|
| `StopCue { cue: "music.calm" }` | `voice("calm")` = the **despawned** entity, `live()` 1, `layer_db` −80 dB, never returns | red |
| `ui.click` on the same bus, `max_voices: 1` | `stopped music.calm: stolen by ui.click` | red |
| a layer's first `Play` refused | `live()` 0 for the rest of the process | red |
| a stinger beside a layer | no steal | green |

## Ranked

1. **A music layer's voice dies and the layer never notices** (three paths above). **The pick.**
2. A `Refusal` variant for a stinger on the wrong bus (`needs-decision/stinger-refusal.md`).
3. A `Refusal` variant that separates "never mapped" from "corrupt" (`needs-decision/undecodable-sound.md`).
4. Quantise a retarget onto the bar.
5. A payload that names the cap.

## After the change

| case | after | green? |
|---|---|---|
| a one-shot on the same bus beside a stem | no `stolen by`, 2 live voices | green (fails before) |
| a voice that died for a reason the plugin did not ask for | restarted, same frame, at the layer's gain | green (fails before) |
| `StopCue` on a layer | stays stopped; `voice()` `None`; `live()` 0 | green (fails before: `voice()` named a despawned entity) |
| a refused layer start | retried when a slot frees | green (fails before) |
| every other case | unchanged | green |
