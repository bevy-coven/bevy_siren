# review.md — iteration 5, three passes

## Pass 1 — function

**The claim.** A `VoiceLimit` refusal names a bus **only when a bus's own `max_voices` is what refused**. A full table
budget and a cue at its own `max_instances` name no bus, because every bus still has room.

**Against the sources.** `VoiceLimit::bus`'s own doc: *"The bus that had no room, **when one did**."* That sentence is
the spec, and the old code contradicted it whenever the cue's polyphony or the table's budget refused. Gregory
§14.5.8.4 sets a group's limit and the engine's limit as separate things a designer tunes; a refusal that conflates
them sends a person to the wrong one.

| cap that refused | before | after |
|---|---|---|
| the table's `max_voices: 1`, two buses each with room | `Some("Ambience")` → ``no voice free on bus `Ambience` `` | `None` → `no voice free` |
| the cue's `max_instances: 1`, its bus at 0 of 8 | `Some("Effects")` → ``no voice free on bus `Effects` `` | `None` → `no voice free` |
| the bus's `max_voices: 1` | `Some("Effects")` | `Some("Effects")` — unchanged, and now the only case that names one |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |

**The first row is the one that matters.** Before, a table-budget refusal accused **`Ambience`** — a bus with a free
slot, and not even the cue's bus. A person reading the Cauldron's log goes to the Ambience row, finds it empty, and
has been told something false with a bus's name attached to it.

**The tests.** `a_full_table_budget_names_no_bus` and `a_cue_at_its_own_polyphony_names_no_bus` fail on the unfixed
tree; `a_full_bus_names_that_bus` passes on both, so a build that stopped naming the bus altogether is red. That
pairing is cases 7–9's rule — a refusal is green when it is the *right* refusal — applied to the payload.

**Verdict: GREAT.** Three caps classified at their own return sites, `Some` used only where the `Option` doc says it
should be, and the non-regressing case pinned beside the two that changed.

## Pass 2 — zoom

**Read:** `evidence/{before,after}/refusals.txt`, `budget.svg` from both trees, and the rendered cue WAVs.

**One defect in my own evidence harness, found and fixed:** the first `budget` run of this iteration refused
everything with `MissingSound { sound: "line" }`, not a voice limit. The probe inserted four **zero bytes** as a
sample, and `start_pending` refuses a sample the mixer cannot decode — so the harness measured the decoder, not the
budget. It now writes a real 16-bit WAV (`wav_bytes`), and the three lines are the three caps.

**A second, unrelated defect the probe exposed and this iteration did *not* fix:** `start_pending` reports a decode
failure as `MissingSound { sound: voice.cue }` — the **cue's** name in a field documented as a *sound key*, so a project
greps its asset list for `footstep.grass` and finds nothing. It is a refusal naming the wrong subject, exactly this
iteration's class, and it is logged as the next item rather than folded in silently.

**The plots.** `budget.svg` is unchanged by this iteration (it counts refusals, not their words) and is identical
across the trees, which is correct and says nothing about the fix; the evidence is the three text lines.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A different shape of run.** The probe forces each cap with the smallest table that reaches it, and the third case
was built to be reached *through a different bus* (`Ambience`) than the cue's own — which is how the wrong name became
visible. The unit tests reach the same three caps by their own routes.

**Can a developer understand it without the code?** `refusals.txt` is three lines: a label, the payload and the
sentence. A reviewer sees two shapes where there was one, and the bus case keeps its name. `CHANGELOG` states the host
consequence — a `match` on `VoiceLimit { bus: Some(_) }` now sees only the bus case — in one sentence.

**What a fresh reader would still ask, and the honest answer:** *"no voice free"* is true but not diagnostic — it does
not say whether the table's budget or the cue's own limit ran out. That needs the cap in the payload, which is a
public API change, so `needs-decision/refusal-names-its-cap.md` carries it with the sentence each option would print.

**Verdict: GREAT.** The earliest pass a fix would return to is pass 2 (a harness that measured the decoder); fixed.
