# eval.md — baseline and the `cues.yaml` table, iteration 35

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 118 + 5 doctests green |
| `cargo test --features serde` | 125 + 5 green |
| `cargo test --features mixer` | 127 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 12 GB free (98 %). Iteration 34's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Case 18 runs `problems_with` on a
**cue** table; **no case checks the score's report against its own doc.**

## The lead, and what it became

Iteration 34's log named the sweep: *"grep every `should` in `src/` and check each has a mechanism."* Five hits, **two real**:

| hit | verdict |
|---|---|
| `MusicLayer::cue`: *"should name the `Music` bus"* | **real** — iteration 34 made that a **refusal**; the doc still said *"should"* |
| `MusicScore::problems`'s doc: four checks | **real** — the function performs **nine** |
| three comments describing history | true as written |

The second is the bigger: the doc listed **four of nine**, so a reader trusted a list and stopped. The two added since —
a gain that is not a number (19) and a layer on the wrong bus (34) — were never added to it.

The absolute claims were tested too, and **held**: an empty cue *only refuses*; a budget of 2 with six unstealable requests
plays **2, refuses 4, never 3**.

## Ranked

1. **The report's doc lists four of its nine checks** — a reader trusts a list and stops. **The pick.**
2. A gain above 0 dB reported as a fault — **a judgement**, written to `needs-decision/above-unity-gain.md`: a layer at
   +12 dB is 3.98×, `sum_db` does not clip, but a score's layers crossfade rather than sum, so any threshold fires on the
   ordinary case. **Not built.**
3. **A Music block in the panel** — deferred a fourth time, budget unchanged (12 px free, ~134 needed).
4. A `Stopped` line for a parent's despawn (blocked on the public-enum decision).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| the report's doc | **nine** checks, each a clause, and which two the plugin enforces | green (four before) |
| `MusicLayer::cue`'s doc | *"must"*, and what happens otherwise | green (*"should"* before) |
| every one of the nine checks | reached by `a_score_reports_every_fault_its_doc_lists` | green |
| behaviour | **unchanged** — no executable line moved | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
