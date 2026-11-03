# eval.md — baseline and the `cues.yaml` table, iteration 19

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 88 + 4 doctests green |
| `cargo test --features serde` | 95 + 4 green |
| `cargo test --features mixer` | 96 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 17 GB free. No open issues. Iteration 18's run has its `review.md`.

## The `cues.yaml` table

All 30 cases green. Case 15/23 drive a score; **no case authors a layer gain that is not a number**, and case 18's
`problems()` mistakes do not include one.

## The lead, and what it became

Iteration 18's log ended with a **measured** lead: `MusicLayer { gain_db: NaN }` reaches `MusicState` unfiltered.
Confirmed, and sized honestly:

| probe | before |
|---|---|
| a score layer at `gain_db: NaN` | `layer_db NaN`, `target NaN`, `Voice.base_db NaN`; the sink gets `Applied −80.0`; **`problems()` empty** |
| a `MusicLayers` move to `NaN` | `layer_db NaN`, `target NaN` |
| a layer started at `SILENT_DB` | `−80.0` (a finite number — must stay) |
| a score with `tempo_bpm: NaN` | **already reported** (`"tempo_bpm is negative or not a number"`) |

The last row is what made this rank 1 rather than 3: the score's report **already speaks** about a non-finite number,
and it named the tempo but not the gain.

**Severity, corrected mid-iteration.** I first expected the layer to be frozen at `NaN` forever. Probing recovery
showed `approach(NaN, −3.0)` returns `−3.0` — the crate's own guard — so a later move recovers it. The finding is *a
silent stem and an empty report until a game happens to move it*, not a permanent loss, and the log says so.

## Ranked

1. **A music layer's gain that is not a number silences the stem with no report** — the one place iteration 18's rule was
   not applied. **The pick.**
2. `Refusal::NotALoop` (iteration 16's item 1; decision file waiting).
3. `CueProblem::BadGain` + the refusal for a bad request (iteration 18's decision file).
4. Re-time a restarted stem to the bar (iteration 15's item 1).
5. Quantise a retarget to the bar (iteration 14's item 1).

## After the change

| case | after | green? |
|---|---|---|
| a score layer at `gain_db: NaN` | `layer_db 0.0`, `base_db 0.0`, `Applied 0.0`; one sentence in `problems()` | green (fails before) |
| a move to `NaN` | `0.0` | green (fails before) |
| a layer started at `SILENT_DB` | `−80.0` — **unchanged** | green before and after (the guard) |
| a good score | `problems()` empty | green (the "must not happen" half) |
| every other case, every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
