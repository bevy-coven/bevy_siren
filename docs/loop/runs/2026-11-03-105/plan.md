# Iteration 105 — plan: report the repeat rule that refuses nothing

## Ranked

1. **`InertAvoidRepeat`** — chosen. Silent wrong output on the crate's headline behaviour (a footstep that machine-guns),
   found by reading, and reachable by a generated table without anyone noticing. `cues.yaml` cases 1 and 2 both speak to
   it.
2. `#28` — needs `serde_json` (absent from the graph) and a `leaf.rs` row: a dependency decision.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` is blocked on a decision only the owner can make. This defect is fixable here, today, and it
sits on the behaviour the crate is named for.

## Design

`src/cue.rs`:

- `CueProblem::InertAvoidRepeat { name }` — a new variant.
- `problems()` reports it when `matches!(cue.repeat, Repeat::AvoidRepeat { avoid: 0 })`.
- The variant's doc gains what `avoid` actually counts (sounds, not indices) and that `0` is `Shuffle` under another name.

`tests/table.rs`:

- the closure asserting *"every problem names its cue"* reads the name rather than enumerating variants — enumerating them
  made adding a kind a compile error in a test about sentences.

## Fields

None. No rate, curve or threshold changes; the rule is a diagnostic over an existing field.

## Tests

- `avoid_repeat_zero_is_reported_because_it_refuses_nothing` — reported for `0`; **not** for `1`, `2`, `4`, `255`, nor for
  `Shuffle` (the refusal that must not fire).
- `a_zero_avoid_window_refuses_exactly_nothing` — the behavioural half: `avoid: 0` and `Shuffle` produce the **same 24-pick
  sequence** from the same seed, and the `avoid: 0` sequence **contains** a repeat while `avoid: 1` contains none.

## Carved off

Making `avoid: 0` behave as `1`. That would silently change a table's behaviour; the crate reports rather than repairs.
