# review.md — iteration 10, three passes

## Pass 1 — function

**The claim.** `SirenMix::db` and `ducked` hold exactly the buses the table has **this frame**; and a table that lists no
buses means the crate's four, for the mix as well as for `answer`.

**Against the sources.** Both fields are documented as *this frame's* (`db`: "each bus's gain this frame"; `ducked`:
"each bus's current duck"), and rule R6 says `SirenMix` is one number per bus so a panel can draw it. A value left from a
previous table is neither.

| measurement | before | after |
|---|---|---|
| `mix.db.keys()` after a swap from 4 buses to 2 | `[Ambience, Dialogue, Effects, Music]` | `[Effects, Music]` |
| `mix.bus_db("Ambience")` for a bus not in the table | **−3.3 dB** | **−80.0 dB** (`SILENT_DB`, its documented answer) |
| `mix.deepest_duck()` | `Some(("Ambience", −12.0))` forever | `None` |
| `buses: []`, a cue on `Effects` | play accepted, **mixed at −80 dB**, no refusal | mixed on the crate's four at 0 dB |
| a bus that stays, one frame after a change | approaches | approaches (unchanged) |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The fourth row is the one that matters**, and my own test found it. Pruning the maps exposed that `update_mix` iterated
`settings.buses` directly while `bus_names()` — read by `answer`, by the panel and by the prune — promises the crate's
four for an empty list. So a project with `buses: []` had every cue accepted and then mixed at **−80 dB**: audible
silence with no refusal, on the default configuration of a table. The pick widened from "stale buses" to "the mix and the
play path disagree about which buses exist", which is the same bug on its other side.

**Verdict: GREAT.** Two halves, each proven to fail alone, and the second one is a silence the crate never reported.

## Pass 2 — zoom

**Read:** the probe output, `evidence/{before,after}/render.txt`, the cue spectrograms and loudness.

**One defect in my own tests, found by the gate:** `a_bus_that_stays_keeps_its_smooth_approach` asserted a level between
the old and the new and got the new one exactly — because the frame that applies the change had `dt = 0` and `approach`
returns its target for `dt <= 0`. The crate's own known trap, in a test I wrote two iterations after writing it down;
fixed by advancing the clock first, which is what the existing music tests already do.

**A dead-code warning fixed on the way:** `tests/helpers.rs::tiny_wav` is used only from the `mixer`-gated module, so a
default build warned it was dead. Gated.

**Audio unchanged:** 16/16 WAVs byte-identical, `ui-cancel` −24.4 LUFS / −12.1 dBFS looped past the gating block,
`mix.wav` −4.3 LUFS / +0.2 dBFS, the attack crop continuous at 35 ms.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** Three tests: the swap (stale buses), the empty list (the mix's own defaults), and the
non-regression (a bus that stays still eases, which a wholesale rebuild would have broken). The third is the one that
catches the obvious wrong fix.

**Can a developer understand it without the code?** `render.txt` now states what `bus_names()` answers for a table and
for a table of none, so the crate's promise is on one line; the `CHANGELOG` says which accessor lied and with what number.

**What a fresh reader would still ask:** whether `DuckClock.started` has the same staleness. It does — its keys are
`(source, target)` pairs and a stale pair is inert once no bus matches — and that is named in the log's next-5 rather
than folded in silently.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a test with the zero-`dt` trap); fixed.
