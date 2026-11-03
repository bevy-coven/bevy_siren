# plan.md — iteration 53: `MusicScore::default()` is the score the file loader gives

## The pick, and why it beats the runner-up

**Pick:** `MusicScore` derived `Default`, so `beats_per_bar` was `0.0`, while serde's field default is
`default_beats_per_bar()` = `4.0`. The same score is a 2.0 s bar from RON and **no bar at all** from `Default`, and
`problems()` reports the crate's own default as broken.

Beats the runner-up (`AvoidRepeat`'s unreported window): that needs the open `CueProblem` enum decision. This is a
**wrong output** — rank 1 — and it needs no API change at all.

## The ranked list

1. **BUG — `MusicScore`'s two defaults disagree.** Shipped this iteration.
2. `cues.yaml` case 31 — a non-finite cue `gain_db` is neutralised silently — **blocked** on the enum decision.
3. `AvoidRepeat`'s window that cannot be honoured is unreported — same decision.
4. **`DuckClock::started` is never pruned** — one entry per fired `(source, target)` for the process's life. Real, but
   `DuckClock` exposes no way to observe it, so a capture could not prove it; logged for a later iteration that adds an
   accessor.
5. `reset_histories` has no caller — a consumer item for `bevy_grimoire`, its own commit.

## The design

**A handwritten `Default`** returning `{ layers: vec![], tempo_bpm: 0.0, beats_per_bar:
Self::default_beats_per_bar() }`. `tempo_bpm: 0.0` stays: it is documented as *"no beat grid: every transition is
immediate, for a score that is a pad rather than a song"*, so zero is a **meaning**, not a missing number — the
opposite of `beats_per_bar`, where zero is forbidden. That asymmetry is why one field's default needed a value and the
other's did not.

## Fields

**No new field, no schema change.** One `Default` implementation, replacing a derive.

## Tests

- **`tests/music.rs::the_default_score_has_the_default_bar`** — ungated: `Default().beats_per_bar == 4.0`, and at 120 bpm
  `bar_s() == Some(2.0)`.
- **`tests/music.rs::the_default_score_matches_the_file_loader`** — gated on `serde`: the RON path and the `Default`
  path agree on `beats_per_bar` and `bar_s()`.
- **`tests/music.rs::the_default_score_is_not_itself_a_problem`** — `Default().problems(&table)` names only *"no
  layers"*, never `beats_per_bar`.
- Both new tests **fail on the old tree** (measured) and pass on the new.
- The whole existing suite, including `tests/music.rs::a_change_waits_for_the_bar`, must stay green: the fix changes a
  default, and a test that relied on the zero would be a test that encoded the bug.
