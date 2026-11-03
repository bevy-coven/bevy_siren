# Iteration 106 — plan: one list of checks instead of three

## Ranked

1. **The doc's false guarantee** — chosen. It is a defect in the crate's own stated contract, on the report an author reads
   when their table is wrong. It ranks as an authoring-ergonomics bug (category 3) and above every open issue, all of
   which are blocked externally.
2. `#28` — needs `serde_json` (absent from the graph) and a `leaf.rs` row: a dependency decision.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` waits on the owner. This is fixable here, today, and it is the difference between a report that
stays correct and one that quietly loses a check.

## Design

`src/mix.rs`:

- `pub const SETTINGS_CHECKS: &[&str]` — all thirteen checks by name, including the `by_db == 0.0` one.
- The doc names the constant instead of counting, and says what the test does with it.

`tests/mix.rs`:

- `problems_names_every_mistake` asserts `SETTINGS_CHECKS.len()` against its own needle list and **names** any check with
  no needle.
- a needle for `by_db == 0.0`, which nothing in that test covered.

## Fields

None. No rate, curve or threshold changes.

## Tests

- the existing `problems_names_every_mistake`, extended: **every name in `SETTINGS_CHECKS` has a needle**, and the two
  counts agree.
- a new assertion that the `by_db == 0.0` sentence appears, since it is the check the drift hid.

## Carved off

Applying the same treatment to `MusicScore::problems` and `CueTable::problems`, which have the same shape. It is a
mechanical follow-up and doing it in the same commit would bury the finding.
