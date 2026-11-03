# review.md — iteration 45, three passes

## Pass 1 — Function: does the report do what its sibling's does?

**The standard.** `tests/music.rs::a_score_reports_every_fault_its_doc_lists` — *"`MusicScore::problems`'s doc lists all
nine checks it performs … a test walks all nine so the list and the code cannot drift apart again."* Written in
iteration 34, after the score's doc named four and the code performed nine.

**The measurement.** Fixed-string search over `tests/` for each of the mix table's nine sentences: five reached,
**four not**. Its doc named six.

**After.** `tests/mix.rs::problems_names_every_mistake` builds one table holding every fault and asserts each sentence;
then shows each is **its own** fault (a table with only the duplicate bus reports exactly one line); then covers the
`NaN` half of the hold/fade sentence, which a `-1.0` hold alone would not; then asserts a good table says nothing.

**Mutation, one sentence at a time** (each `out.push` removed in turn, test run, restored): `master_db` → FAILED,
`a bus has no name` → FAILED, the bus gain → FAILED, hold/fade → FAILED. All four previously-unreached sentences are
now load-bearing.

**Verdict: GREAT.** The report's nine sentences are all reached by a test, and the doc lists nine.

## Pass 2 — Zoom: read the crops closely

Read: `evidence/panel/{clean-96,broken-96,faults-96,broken-mix-40,clean-30,broken-30}.png`, and the shipped
`docs/media/mixer.png`.

**A defect found and fixed.** The first `SIREN_MIX=broken` capture showed the block drawing the two mix faults *and* the
log's last line clipped: measured, the lowest ink sat at **`y = 719` of 719 — 0 px**. The block costs two lines it did
not before. The log's line count now follows the block: `6 - (drawn - 1)`.

**A regression found by the same rule.** With the log counting the block properly, `SIREN_FAULTS=many` still clipped at
**0 px** — but the cause was elsewhere: the cue list drew **twelve** rows and pushed the `Problems` block *and the whole
log* off the bottom, so the log's heading was not on screen at all. The list is now bounded at `CUES_SHOWN = 8` with
`and 6 more`, and the log's count subtracts that row too. Measured after: clean **9 px**, broken **10 px**,
`faults=many` **10 px**.

**No clipping at 1280.** The shipped still keeps its 9 px margin with eight cue rows, `Problems: none` and six log lines.

**Verdict: GREAT.** After two fixes, no capture clips.

## Pass 3 — Fresh eyes: a different table

Three tables were captured and read: the clean one, `SIREN_MIX=broken` (NaN master, negative duck hold), and
`SIREN_FAULTS=many` (14 cues, six faults). Each reads differently, and each says which file the fault is in:
`master_db is not a number` is the mix table's; `cue \`x\` …` is the cue table's. Nothing is merged into an
unattributable list.

**Can a developer understand it without the code?** Yes — `Problems` answers one question (*is this table fit to run*),
and every sentence names its field and its value. `docs/guide/buses.md` already says the report is what a person reads
on an editor's status line.

**Verdict: GREAT.**
