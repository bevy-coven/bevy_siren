# review.md — iteration 46, three passes

## Pass 1 — Function: does the report name the thing that happened?

**The measurement.** `Duck::new("Dialogue","Music").by_db(-8.0)` — no hold, no fade — starting a voice on `Dialogue`:

| | `SirenMix::ducked["Music"]` on the frame the voice starts |
|---|---|
| the bare rule | **`Some(0.0)`** |
| the same rule with `.hold_s(0.25).fade_s(0.6)` | **`Some(-8.0)`** |

The rule fires (`fire_all` matches its `source`), is recorded, and lowers the bus for **no time at all** — S2's
`active_duck_db` returns `0.0` past the hold when `fade <= 0.0`. The crate's `default_ducks()` sets both fields, which
is why the pair every project starts from is unaffected and the hazard never surfaced.

**After.** `problems()` emits a tenth sentence and the panel draws it:
`the duck on \`Music\` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s`
(`evidence/panel/broken-40.png`).

**The guard.** `by_db < 0.0`, because a rule that **raises** already has its own sentence and would otherwise be
reported twice for one mistake.

**Verdict: GREAT.** The behaviour is measured and unchanged (deliberately — see the carve), and the mistake is named.

## Pass 2 — Zoom: read the crop closely

Read: `evidence/panel/broken-40.png`, plus the whole-repo `docs/media/mixer.png` regenerated later.

**A defect found in this iteration's own test, before it shipped.** The first version of the big fault table had
`Duck::new("Nope","Music")` and `Duck::new("Music","Nope")` with no hold or fade — so the *new* sentence fired from a
rule whose real fault was a **missing bus**, and the assertion would have passed for the wrong reason. Caught by
running the test after removing the accidental triggers: the report listed nine sentences and not the tenth. Every rule
in that table now carries `hold_s`, and a dedicated `Duck::new("Music","Music")` row carries the tenth.

**Two compile errors in my own test, both recorded**: `app` shadowing the `app(..)` helper, and `ducked` holding every
bus at `0.0` where I asserted `None`. The second is a real lesson about `SirenMix` and is in the test's comment.

**No clipping.** The broken-table capture measures **9 px** of margin — the last band `(692, 710)` of 719 — because the
block draws three sentences and the log's count already accounts for it (iteration 45's rule).

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: a different table

The panel was read with three tables: clean (no sentence), `SIREN_MIX=broken` (three faults, the tenth among them), and
the shipped still. The clean table's `Problems: none` is the control: the new sentence **only** appears for a rule that
actually has neither field, so it cannot become noise.

**Can a developer understand it without the code?** Yes — `set hold_s or fade_s` names exactly what to type, and
`docs/guide/buses.md` already says a duck *"holds for `hold_s`, then recovers over `fade_s`"*; a rule with neither runs
for no time, which is the sentence that was missing.

**Verdict: GREAT.**
