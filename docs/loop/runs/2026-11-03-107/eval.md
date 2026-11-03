# Iteration 107 — eval

## Baseline before anything

Nothing red: 199 / 6 / **436 semantic** / **460 all-features** / 109 lib / 6 doc, clippy and fmt clean, docs build.

## The cue cases

All 33 of `cues.yaml` green, unchanged. This iteration touches the music score's **report**, not the music.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| each documented music fault fires | **green, already** | 9 needles + 1 dedicated fixture — a test existed and was correct |
| the **set** is exactly the documented one | **RED before** | nothing asserted it: adding a tenth check to `problems` left the suite green |
| the doc's *"this list is the whole of them"* | **RED before** | prose, with nothing enforcing it |
| the doc names the list instead | **green now** | `MUSIC_CHECKS` in the doc's first line |
| adding a tenth check **and its name** fails | **proved** | `9 != 10`, naming the whole list; then reverted |
| a name with no check is caught | **proved** | `MUSIC_CHECKS.contains` assertion in the dedicated-fixture loop |
| 436 semantic / 460 all-features | **green** | the existing suite is the proof nothing else moved |

## A correction to the previous iteration's criticism

Iteration 106 recorded: *"`MusicScore::problems` claims the same shape and is one inspection away from the same drift."*
**Half right.** A test *did* exist, with nine needles, and its own doc records that the list once said four while the code
performed nine. What did not exist was the guarantee about the **set** — the counts agreed by hand, and a tenth check
passed silently.

## The item

`music::MUSIC_CHECKS` (nine names), re-exported at the crate root; the doc names it instead of counting; the test walks it.
