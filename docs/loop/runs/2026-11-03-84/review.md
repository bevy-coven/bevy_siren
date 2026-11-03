# Iteration 84 — review

## Pass 1 — Function: is #21's last criterion met?

**Critique.** The criterion is *"both values are visible per asset and survive an index round trip"*, and the previous
three were already green from iteration 77:

| # | Criterion | State |
|---|---|---|
| 1 | integrated within **0.1 LU** of the reference | **green** — −23.0 ±0.1 against `ffmpeg`'s −23.0 |
| 2 | a clip shorter than a gating block handled **explicitly** | **green** — `None` in memory, `-` on disk |
| 3 | true peak within **0.1 dB** | **green** — −20.00 dBFS against −20.0 |
| 4 | both survive a round trip | **green** — this iteration |

**Critique of the format.** The section writes `-` for an absent loudness, and that is the whole design. ADR 0003
measured that for a 0.2 s clip `ffmpeg` reports `−70.0 LUFS` — **the floor presented as a level, and also a real level** —
and `ebur128` reports `-inf`. A storage format that writes a number there destroys a distinction the in-memory type was
carefully built to keep, so the file keeps it too.

**Verdict: GREAT.** #21's four criteria are met, three of them measured against a reference.

## Pass 2 — Zoom: the two defects the tests caught

**Critique.** Both were found by the new tests rather than by reading the code, and both are worth naming.

1. **The marker was parsed as a record.** `provenance_and_loudness_sections_coexist` failed with
   `Record { line: 11, what: "a provenance record names no hash" }`: the `loudness 1` line arrived while the provenance
   section was open, and the section parser consumed it. Fixed by recognising **markers before parsing any record**,
   which also means the sections may appear in any order — a property the writer does not need but a hand-edited file
   will.
2. **My own test asserted behaviour the format does not have.** `a_non_finite_level_is_written_as_absence` expected
   `INFINITY` to round-trip through `-`. The parser requires a number in that column and **refuses the whole file by
   name** — `"a loudness record is not <hash> <lufs|-> <dbtp> ..."`. That refusal is right: substituting `0.0` would
   look like a measured silence, which is the class of lie this section exists to prevent. The test was renamed and now
   asserts the refusal.

**The round trip is read closely.** `None` comes back `None`; the peak comes back intact; `too_short()` and `silent()`
**still disagree** after a write and a read, so the two-count distinction survives the file and not just memory.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what is different for a consumer?

**Critique.** Nothing is required of a consumer. `Index` gains a field defaulting to empty, `to_text`/`load` accept every
previous file unchanged, and no public type changed this iteration — unlike iteration 83, where `Match` lost `Copy`.

An index with no loudness is **byte-identical** to what the crate wrote before, asserted on the text rather than assumed.
An index with loudness and provenance carries both, in either order.

**The honest limit**, stated rather than glossed: loudness is **stored and round-tripped, not yet measured during
indexing**. `loudness::measure` is built and tested (iteration 77), and wiring it into `indexer::build` belongs with the
same scan-side step that joins provenance's file names to content hashes. The criterion asks that the values be
*visible per asset and survive a round trip*, which is what this iteration proves; producing them during a scan is the
next step and is named in the log.

**Verdict: GREAT.**
