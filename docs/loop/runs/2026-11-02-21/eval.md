# eval.md — baseline and the `cues.yaml` table, iteration 21

## Baseline (recorded before any change, 2026-11-02, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 96 + 4 doctests green |
| `cargo test --features serde` | 103 + 4 green |
| `cargo test --features mixer` | 104 + 4 green |
| `cargo test --doc` | 4 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 16 GB free. No open issues. Iteration 20's run has its `review.md`.

## The `cues.yaml` table

All 30 cases green — **and that is the finding**: case 18 lists four table mistakes and case 31 did not exist, so no row
asked whether `problems()` covers a cue's own numbers. It does not, for `gain_db`.

## The lead, and what it became

Iteration 20's lead was `serde`. Probed, and **clean**:

| probe | result |
|---|---|
| `gain_db: NaN` in RON | parses; `to_ron` writes `NaN`; `from_ron(to_ron(x))` returns `NaN` — exact |
| an unknown field (`volume: 3.0`) | **refused**, naming the field and the accepted ones (`deny_unknown_fields`) |
| `max_instances: 65535` | parses; `problems()` says nothing — no rule binds a cue's cap to the table's budget |

The same probe found the real gap, in `problems()`' coverage of one struct:

| field | reported? |
|---|---|
| `Cue::pitch` (inverted) | **yes** — `InvertedPitch` |
| `Cue::attenuation` (negative) | **yes** — `BadAttenuation` |
| `Cue::gain_db: NaN` | **no** |
| `SoundSlot::gain_db: NaN` | **no** |

## Ranked

1. **`problems()` covers two of a cue's three numbers and not the third** — with the fix blocked on a public-enum
   decision. **The pick**, shipped as a red case + corrected docs.
2. `Refusal::NotALoop` (iteration 16's item 1; decision file waiting).
3. `CueProblem::BadGain` (this iteration's evidence added to the same decision file).
4. Re-time a restarted stem to the bar (iteration 15's item 1).
5. Quantise a retarget to the bar (iteration 14's item 1).

## After the change

| case | after | green? |
|---|---|---|
| case 31 (new) | `state: red`, `blocked_on:` the decision file; two of four numbers reported | **red, recorded** |
| the guide's list of checks | says which number is not checked, why, and what to do | corrected |
| every existing case | unchanged; no `src/` change | green |
