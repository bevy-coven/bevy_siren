# Iteration 122 — plan

## The item

`needs-decision/exact-pitch-draw.md`, option **B**, and the same rule where the picker already knows its answer: **a
draw that cannot change what is heard is not taken.** Two places in the play path spend one:

| where | when it cannot decide | before | after |
|---|---|---|---|
| `answer`, `src/play.rs:661` | the cue's pitch range is **exact** (`low == high`), or **inverted** (`low > high`, which `problems()` reports and `at` answers `0.0` for) | 1 draw | **0** |
| `VariantPicker::pick_from` / `pick`, `src/model.rs:465`, `:501` | the cue has **one sound**, or the avoided window leaves **exactly one** candidate (an alternating two-variant cue after its first play) | 1 draw | **0** |

`pick_from` already returns early for `len == 1`; it does **not** for the forced-pick case, and `pick` mirrors it. A
three-variant `avoid: 1` cue still has two candidates every play, so it still draws: **the fix must not turn a real
choice into a forced one.**

## Why this and not the runner-up

It is wrong output on `docs/guide/cues.md`'s own sentence (*a project's soundscape is a function of its seed and its
play order* — written as a limitation), it is measured (the probe in `eval.md`), the rule has a source (randomness is
partitioned by *use*, and a use that is not a decision takes nothing: L'Ecuyer et al. 2002; Clark 1990; Kämpke 1989),
and **its release-note cost is paid**: iteration 121's stream change has not shipped, so a project is re-rolled once,
not twice (`seed-scramble.md` already recommends exactly that). The runner-up — authoring refusals that print nowhere —
is a design question that needs a measurement before a build, so it does not fit one iteration's window.

## The design

**`PitchRange` gains nothing**; `at` is unchanged. The decision is at the draw:

```rust
// A range with one detune in it — or an inverted one, which `problems()` reports and `at` answers 0 for — has
// nothing for the stream to decide.
let mut pitch = if cue.pitch.is_inverted() || cue.pitch.low == cue.pitch.high {
    cue.pitch.at(0.0)
} else {
    cue.pitch.at(choices.rng.next_f32())
};
```

and in the pickers, the draw goes inside a `choices.len() > 1` branch, remembering the single candidate without
drawing.

**Why `at(0.0)` rather than `low`:** it is the same function the drawn path calls, so an inverted range still answers
`0.0` and a finite exact range still answers its value — one rule, not a second one that can disagree. `low == high`
with `low = NaN` cannot reach it: `is_inverted()` is true for a non-finite bound, so the `at(0.0)` branch is chosen and
`at` answers `0.0`, exactly as today.

**No new field.** `PitchRange`'s two floats already say everything; a `draw: bool` would be a second way to say *"low
equals high"*.

## Modules touched

- `src/play.rs` — `answer`'s pitch draw (one expression).
- `src/model.rs` — `pick_from` and `pick`: the draw becomes conditional; docs for both say which pick is free and why.
- **No public type changes**; `VariantPicker`'s behaviour on a forced pick is the only semantic change, and
  `pick`/`pick_from` are public, so it is recorded as a release note with `PitchRange`'s.

## The tests that will prove it

| test | where | red on the unchanged tree because |
|---|---|---|
| `an_exact_pitch_range_leaves_the_stream_alone` (rewritten from `an_exact_pitch_range_still_draws_from_the_stream`) | `tests/play.rs` | it asserts equality where the tree gives a difference |
| `a_forced_pick_is_not_a_draw` | `tests/model.rs` | `pick_from` with one candidate advances the `Rng` |
| `pick_with_one_candidate_is_not_a_draw` | `tests/model.rs` | `pick` **does not** return early for `len == 1` — both spend |
| `an_exact_pitch_cue_leaves_a_shuffle_cue_alone` | `tests/play.rs` | the burst's plays move the observer |
| `a_three_variant_window_still_draws` | `tests/model.rs` | the refusal that must **not** happen: two candidates still draw, and the pick still differs across draws |

Plus the whole `cues.yaml` table and a determinism test unchanged (the same seed still replays field for field).

## Predictions

In `research.md` (P1 – P9), written before any code.

## Carried off

- A stream per use (`exact-pitch-draw.md` option C), a stream-version setting, and the carried top-5 items.

## What this does not touch

The variant picker's rule, `Repeat`, the budget, the buses, the mixer, the semantic path, and every sound: a cue whose
answer depended on a number draws from the same stream in the same order.

## The baseline blocker, and what this iteration does about it

`cargo test --features semantic` failed **only** with `No space left on device` at 2.1 GB and passed alone. The matrix
script is now `docs/loop/matrix.sh` with the all-features test step split (`--tests -j 1`), which is what let iteration
121's failing step pass at 2.8 GB. **The dependency artifacts are 9.8 GB of debug deps and are not this iteration's to
delete**; the footprint is recorded in `perf.md` and in the log's top 5, and a `[profile.dev] debug = 1` (or
`line-tables-only`) is recommended for the owner's decision — **not** applied, because it changes every consumer's
`cargo build` time in this repository and is a build-config decision, not a crate one.

## The stale scratch binaries

`target/debug/examples` holds ~0.9 GB of binaries of scratch examples from iterations 110 – 113 (`diag`, `probe`,
`lcheck`, `zzdump`, `zz_demo_library`) whose sources are gone — `cargo clean -p` cannot see them. **They predate this
session, so they are the owner's to remove**, and they are named in the log. The `scratch_*` files this iteration makes
are deleted the moment their evidence is saved.
