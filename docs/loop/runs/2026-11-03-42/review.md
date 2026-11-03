# review.md — iteration 42, three passes

## Pass 1 — Function: does it do what the source and the case say?

**The cases.** `cues.yaml` 7: *"a third play takes the oldest back rather than sounding a third"*, `green_when: Two live
voices; one entry whose reason starts `stolen by`"*. Case 8: *"a bus cap holds when three requests arrive in one
frame … Two voices live, one steal logged."* S1: `max_instances` is *"a budget on plays"*.

**Measured on the isolated `poly` run** (`SIREN_KEYS=poly`, frame 40 — an empty `Effects`):

| Where | Text |
|---|---|
| cue row | `footstep.grass  Effects  once  +0.0 dB  priority 10  x2` |
| bus row | `Effects  +0.0 dB  2 / 2 voices` |
| log | `0.37s played \`footstep.grass\` (step_01) …` |
| log | `0.37s stopped \`footstep.grass\`: stolen by footstep.grass **(cue)**` |
| log | `0.37s played \`footstep.grass\` (step_02) …` |

Three requests, **one** steal, **two** live voices, Effects at `2 / 2` — case 7's assertion, and the row's `x2` is the
number that produced it. The sentence names **`(cue)`**, which is S1's cap and the one an author would raise.

**Second case, the bus.** `SIREN_KEYS=steal` (`zoom-bus-cap.png`): `stopped \`engine.idle\`: stolen by sword.hit (bus)`.
Before this iteration the two sentences were `stolen by footstep.grass` and `stolen by sword.hit` — one said nothing
about which of three numbers was full, the other said nothing either.

**Verdict: GREAT.** Case 7's assertion is now readable, and the sentence distinguishes two caps that were one string.

## Pass 2 — Zoom: read the crops closely

Read: `evidence/panel/{poly-95,poly-60,poly-only,steal-40,faults-30,zoom-poly-log,zoom-poly-caps,zoom-bus-cap,zoom-faults}.png`
and the shipped `docs/media/mixer.png`, `mixer.gif`.

**The column's alignment.** `{poly:>3}` right-aligns in three columns, so `x2`, `x1` and `all` share a right edge. In
`poly-95.png` the eight cue rows keep their columns and the spatial clause still starts at the same x — no row wrapped,
measured: the longest (`bang … inverse at 8 m -6.02 dB`) inks to x≈990 of 1296 usable.

**A defect this pass found and fixed.** The first default script put `poly` **second**, so the shipped still — captured
at frame 60 — showed six log lines of nothing but footsteps and **hid the stinger's duck, the dialogue duck and the
spatial play**, which are three iterations' worth of work. The picture was correct and worthless. `poly` now runs last,
the still is captured at **140**, and the same frame carries all four stories: `duck -6.0 dB by stinger`,
`duck -2.2 dB by dialogue.greeting`, `inverse at 8 m -6.02 dB`, and `x2` with two `(bus)` steals.

**A second defect, found by counting.** In the default run the log showed **two** `stolen by footstep.grass` lines for
one three-request step. Arithmetic (`room_for` per request: 0→1, 1→2, 2→2) says **one** cue steal; the second line was the
**bus** cap taking a different footstep voice. That is exactly the ambiguity this iteration removed — after the change
the isolated run shows **one** `(cue)` line, and the default run's two lines say `(bus)` and `(bus)`, which is true.

**The `0` branch.** `faults-30.png` under `SIREN_FAULTS=many` shows `unlimited … all <- problem`, paired with
`UnlimitedInstances` in the `Problems` block — the row does not invent `x0`.

**Verdict: GREAT.** After the two fixes: no wrap, no clip, and the column's number is the number the log proves.

## Pass 3 — Fresh eyes: a different seed and a different script

`SIREN_SEED=4242` is not needed to vary this: the column is a property of the **table**, so the stronger fresh-eyes test
is a **different script and a different table** — `SIREN_KEYS=poly` (one step), `SIREN_KEYS=steal` (bus cap only), and
`SIREN_FAULTS=many` (14 cues, including `max_instances: 0`). All three were captured and read.

- **One step:** the `(cue)` sentence, 2 voices, `2 / 2` — the column's `x2`, alone.
- **Bus only:** the `(bus)` sentence — the other cap, alone.
- **Faults table:** `all` for the unlimited cue, `x1` for four new cues, and the block still fits at 14 rows.

**Can a developer understand it without the code?** Yes: `x2` reads as a count, `all` as "no limit", and `(cue)`/`(bus)`
name which number to raise — `docs/guide/buses.md` uses the same three words for the same three caps. Nothing needs the
source. The GIF now opens at frame 45 and runs through the stinger and the poly step, so both sentences appear.

**Verdict: GREAT.**
