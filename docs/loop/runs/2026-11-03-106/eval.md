# Iteration 106 — eval

## Baseline before anything

Nothing red: 199 / 6 / **436 semantic** / **460 all-features** / 109 lib / 6 doc, clippy and fmt clean, docs build.

## The cue cases

All 33 of `cues.yaml` green, unchanged. This iteration touches the mix table's **report**, not the mix.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| the doc's count matches the code | **RED before** | prose said **twelve**, code performed **thirteen**, test asserted **eleven** |
| the doc's stated guarantee holds | **RED before** | *"this list and the code cannot drift apart"* — nothing enforced it |
| the `by_db == 0.0` check is covered by the list's own test | **RED before** | its sentence appeared in **no** needle and **no** doc bullet; only in three separate places elsewhere in `tests/mix.rs` |
| every check has a needle or a named dedicated fixture | **green now** | `ASSERTED_ELSEWHERE`, with an assertion that each entry is still a real check |
| adding a check without a needle fails | **proved** | appended a fourteenth name to `SETTINGS_CHECKS`: the test failed naming it, then reverted |
| 436 semantic / 460 all-features | **green** | the existing suite is the proof nothing else moved |

## How the defect was found

By **counting**. `grep -c 'out.push|said.insert'` on `problems` returned 15 sites against a doc claiming twelve; reading
them showed 13 distinct checks, and cross-checking the test's needles gave 11. Three numbers, none of them equal, and the
doc asserted they could not disagree.

## The item

`mix::SETTINGS_CHECKS` (13 names, the one list), the doc rewritten to point at it, `tests/mix.rs` extended to walk it, and
a `by_db(0.0)` duck added to the fixture so the check the drift hid is finally exercised there.
