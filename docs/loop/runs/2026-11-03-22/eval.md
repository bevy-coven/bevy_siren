# eval.md — baseline and the `cues.yaml` table, iteration 22

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 97 + 5 doctests green |
| `cargo test --features serde` | 104 + 5 green |
| `cargo test --features mixer` | 105 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 15 GB free. No open issues. Iteration 21's run has its `review.md`.

## The `cues.yaml` table

All 31 cases checked; **case 31 is red by design** (`state: red`, `blocked_on:` the gain decision), and the other 30 are
green. `cues.yaml` case 1 is the case this iteration is about — *"Variants do not repeat back to back"* — and it was
green only because its cue has **three distinct** keys.

## The lead, and what it became

Iteration 21's lead was `SirenSounds` and colliding slot keys. Probed:

| probe | result |
|---|---|
| one key, two files (across cues) | **not representable** — one key maps to one handle by design (`SirenSounds`' module doc) |
| the same key twice in one cue | `CueTable::sounds()` deduplicates it; `problems()` says nothing; the **picker sees two indices** |
| `load_table` with a key of `../outside` | joins to `audio/../outside` — a separate item, named in the log |

The second is rank 1: `VariantPicker` refuses **indices**, so `["a", "a", "b"]` under the default
`AvoidRepeat { avoid: 1 }` played the same recording **three times running** (`… b a a a b`), and `["a","a","a","b"]`
played four. That is the machine-gun footstep `Repeat` exists to kill, on by default, with no report.

## Ranked

1. **`AvoidRepeat` refuses an index while a player hears a sound** — the crate's headline feature, defeated by a
   copy-pasted line, silently. **The pick.**
2. `Refusal::NotALoop` (iteration 16's item 1; decision file waiting).
3. `CueProblem::BadGain` (iteration 18; `cues.yaml` case 31 red).
4. `CueProblem` for a duplicate key (new file this iteration).
5. `load_table`'s path join (named, not built).

## After the change

| cue, `AvoidRepeat { avoid: 1 }` | after | green? |
|---|---|---|
| `["a", "a", "b"]` | **0** adjacent repeats, both sounds play | green (3 before) |
| `["a", "a", "a", "b"]` | **0** adjacent repeats | green (8 before) |
| `["a", "b"]` (control) | 0 — unchanged | green |
| `["a"]` (one sound, guard) | 15 — unchanged | green |
| `pick` without keys | same sequence as `pick_from` over distinct keys | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
