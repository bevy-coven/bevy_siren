# Iteration 79 — review

## Pass 1 — Function: does the ADR meet #5's acceptance criteria?

**Critique.** Four criteria, each either met or not:

| Criterion | Met? |
|---|---|
| an ADR exists at `docs/adr/0004-vector-store.md` | **yes** |
| file size for a stated library size computed, not guessed | **yes** — computed at 500, 5 000 and 50 000 clips in three encodings, from a measured per-record cost |
| the content-hash key and its collision policy specified | **yes** — a four-row table covering rename, re-encode, duplicate bytes and collision |
| a query with no writable filesystem stated as supported or not | **yes** — supported, and the mechanism is named (`File::open` + `read_to_end`, no `MAP_SHARED`, no lock) |

**Critique of the decision.** Rejecting SQLite is the load-bearing choice, and the *reason* has to be the measurement
rather than taste: `rusqlite` pulls C into a crate whose dependency list a test enforces, to solve write amplification
on a **10.3 MB** file rewritten at a rate of once per incremental update. That is a real trade and the ADR states what
is given up — no transactions, and a whole-file rewrite that becomes wrong at 50 000 clips.

The **`f16`** decision is the one worth flagging: the measurement says it halved the file and preserved the top 10
exactly, and the ADR still declines it. That is correct rather than timid, because `rank::top_k` compares against a
`floor` and a one-query top-10 agreement says nothing about behaviour at that boundary. It is deferred to `V-01`, which
is where a labelled set would settle it.

**Verdict: GREAT.**

## Pass 2 — Zoom: are the numbers airtight?

**Critique.** Every size in the ADR is arithmetic on a **measured** quantity, and the arithmetic is checked here:

- 512 floats × 4 bytes = **2 048**, + 16 hash = **2 064** bytes/record. × 5 000 = 10 320 000 ≈ **10.32 MB**. The ADR
  says 10.30 MB — the difference is the `Vec` length prefix it correctly does not count.
- `f16`: 512 × 2 = 1 024 + 16 = **1 040** × 5 000 = 5 200 000 = **5.20 MB**. Matches exactly.
- Text: 6 376.8 × 5 000 = 31 884 000 ≈ **31.88 MB**. Matches.
- 50 000 scales all three by 10 exactly, which the table does.

**The extrapolation is labelled.** Query time at 50 000 is stated as *"linear scaling, labelled as extrapolation"* in
`research.md` and *"~0.5–19 ms"* in the ADR — a range, not a point, because the measured range at 5 000 is itself wide
(1.87 ms cold vs 0.05 ms warm) and pretending otherwise would be a false precision.

**One number is weaker than it looks.** The `f16` top-10 agreement is **one query against one synthetic library**. The
ADR says so explicitly rather than generalising it, and it is why `f16` is deferred — a measurement that cannot support
a conclusion is recorded as a measurement, not as evidence.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: does a reader get what they need, and is the claim reproducible?

**Critique.** A reader arrives with the issue's framing — "5 MB vs 40 MB SQLite" — and the ADR's second table
**corrects it in one line**: the 40 MB figure is the format the crate already has. That is the single most useful thing
in the document and it is placed before the decision, which is where a reader needs it.

For the next agent: the ADR names what implementing it changes (`semantic::index` gains a binary sibling to
`to_text`/`load`, same `MAGIC` discipline) and what it does **not** decide (the loudness section, which stays in
`needs-decision/loudness-in-the-index.md` — a decision *inside* this store, not folded into this ADR).

**The honest gap.** Nothing was compiled or run as a test: the volume had 148 MB free and one 7 KB write failed with
`ENOSPC` before reclaiming 326 MB. Every number here came from in-memory Python measurements and from the crate's own
format definition, both reproducible from `evidence/store-sizes.txt`. The change is docs-only, so nothing could
regress — but **unrun is unrun**.

**Verdict: GREAT**, with the baseline gap stated in `eval.md` and the disk named as the environment's fault rather
than the change's.

## The blocker, recorded for the fifth time

`~/.cache/huggingface` holds **50 GB**, of which **48 GB is two FLUX image-model checkpoints** (33 GB and 15 GB) owned
by the user's image tooling, not by this repo. Deleting either recovers more than enough for a full release build. It
is not this iteration's decision to make.
