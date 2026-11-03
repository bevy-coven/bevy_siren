# Iteration 107 — plan: one list for the music score too

## Ranked

1. **`MUSIC_CHECKS`** — chosen. It is iteration 106's recorded criticism, it is the same defect class, and the repair is
   already proved on the sibling type. A doc that says *"this list is the whole of them"* with nothing enforcing it is the
   authoring-ergonomics bug this loop ranks above the (externally blocked) open issues.
2. `#28` — `serde_json`, absent from the graph, plus a `leaf.rs` row: a dependency decision.
3. `#35` — labelled real recordings: a person's work.
4. `#32`–`#34`, `#38` — `bevy_grimoire`.
5. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** `#28` waits on the owner. This is fixable here, today, and I measured the gap rather than assuming it.

## Design

`src/music.rs`:

- `pub const MUSIC_CHECKS: &[&str]` — nine names, at module scope, re-exported like `SETTINGS_CHECKS`.
- The doc names the constant and drops the count.

`tests/music.rs`:

- `a_score_reports_every_fault_its_doc_lists` asserts `needles.len() + dedicated == MUSIC_CHECKS.len()`.
- **A dedicated-fixture list**, because `the score has no layers` needs an empty score — the same shape iteration 106
  needed for the over-budget bus.

## Fields

None.

## Tests

- the existing nine-needle test, extended to compare against `MUSIC_CHECKS`.
- **proved by adding a tenth check**: the suite must fail naming the list, then be reverted.

## Carved off

`CueTable::problems` has the same shape and is the third of three. Doing all three in one commit would bury the finding,
and 106/107 have each been a small, legible change.
