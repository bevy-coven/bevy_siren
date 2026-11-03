# review.md — iteration 48, three passes

## Pass 1 — Function: does the report name the number that does nothing?

**The measurement.** `SirenSettings { max_voices: 4, buses: [BusLevel::new("Effects").max_voices(16)] }`:

| | `bus_voice_limit("Effects")` | `problems()` |
|---|---|---|
| before | `4` | `[]` |
| after | `4` | `bus \`Effects\` asks for 16 voices but the table's max_voices is 4, so it mixes 4` |

The capture agrees in three places: the sentence, the bus row `Effects 2 / 4`, and the header's `4 voices` — the table's
budget, not the bus's wish.

**The two must-stay-quiet cases**, asserted in the same test: a bus at `0` (the sentinel, asking for the table's) and a
cap **equal** to the table's (4 of 4 asks for what it gets). Without them the sentence would be noise on every ordinary
table — and the crate's own default table, whose buses ask for `0` and `2` under `12`, reports nothing.

**Verdict: GREAT.** A number that did nothing is now named, and the two cases that must not fire do not.

## Pass 2 — Zoom: read the crop closely

Read: `evidence/panel/overbudget-40.png`, and the shipped `docs/media/mixer.png` regenerated later.

**A design decision made from looking, not from guessing.** `SIREN_MIX=broken` already carries three faults and the
`Problems` block caps at **three sentences** — iterations 46 and 47 chose those deliberately. A fourth would be the
invisible `and 1 more`, so the capture could not prove the sentence. `SIREN_MIX=overbudget` gives this fault **its own
table**, and the block becomes one sentence: the whole block is the thing being proved.

**The sentence is 71 characters**, one line in the column, and the log below it keeps **six** lines because the block
cost one — iteration 45's count rule doing its job on a table it was not written for.

**A duplicate I created and removed.** I appended a second near-identical clamp test before noticing last iteration's
still there; the older one was deleted, and the survivor rewritten to assert the sentence rather than the silence.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: two tables, one sentence

The panel was read with the **clean** table (`Problems: none` — the control), `SIREN_MIX=broken` (three sentences, the
tenth and eleventh among them) and `SIREN_MIX=overbudget` (one sentence). Each block says only what its table has wrong.

**Can a developer understand it without the code?** Yes: the sentence gives the bus, the number written, the number it
gets, and the reason (`the table's max_voices is 4`) — an author knows both which field to change and which one would win
if they only changed the bus.

**Verdict: GREAT.**
