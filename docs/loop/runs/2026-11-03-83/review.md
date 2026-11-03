# Iteration 83 — review

## Pass 1 — Function: are #14's blocked criteria now met?

**Critique.** Two were blocked on a decision; implementing the recommendation settles both:

| # | Criterion | Test | Why it is not vacuous |
|---|---|---|---|
| 1 | explicit unknown, distinguishable from recorded empty | `an_empty_licence_survives_the_file_distinguishably_from_an_absent_one` | asserts `Some("")` vs `None` **after a write and a read**, so it fails if the format collapses them |
| 3 | a search result reports licence and source | `a_search_result_carries_its_provenance` | asserts the fields on the returned `Match`, and that an unrecorded asset still carries a value |
| 4 | unknown never presented as permissive | `a_result_cannot_be_built_without_provenance` | the field is not `Option`, so the failure is a compile error rather than a forgotten lookup |
| 2 | a sidecar's fields reach the index | **still half** | the reader and the store exist; joining a sidecar's *file name* to a *content hash* needs the scan |

**Critique of the format.** Three decisions carry it, and each is forced rather than chosen:

- The section sits **inside the checksummed body**, so a hand-edited licence is a corrupt file rather than a silent
  change to which assets may ship.
- An index with no provenance writes **no section**, making the new format a strict superset — `MAGIC` does not move and
  an old file is a valid new file.
- Fields are **escaped**. Without it, a pack called `kenney sounds` splits into two fields and a licence of `-` reads as
  *nothing recorded* — the same conflation the type refuses in memory, reintroduced by the file format.

**Verdict: GREAT.**

## Pass 2 — Zoom: the round trip, read closely

**Critique.** A format change is only as good as what comes back, so the tests assert content rather than shape:

- **A space inside a pack name survives.** `"kenney sounds"` is written, read, and compared equal — the case an
  unescaped space-delimited format gets wrong.
- **Recorded-empty and absent stay different through the file.** `Some("")` comes back `Some("")`; `None` comes back
  `None`; and `is_recorded()` differs between them afterwards.
- **Truncation is caught in the new section too.** The test re-checksums the body so the record count is the *only*
  thing wrong, which means it fails on the count check rather than on the checksum — the specific rule under test.
- **`?unknown` is not `-`.** A `Source::Unknown` round-trips as `Some(Source::Unknown)`, distinct from `None`, which is
  the in-memory distinction carried to disk.

**A defect this pass found**: `a_search_result_carries_its_provenance` inserted hashes `1` and `2` into `index_with(2)`,
which already holds `0x1000` and `0x1001` — so four vectors ranked and the assertion read `left: 4097, right: 2`. **The
test was wrong, not the code.** It now builds a fresh index. Worth naming because a fixture that silently adds to a
fixture is how a test starts passing for the wrong reason.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does a consumer have to do differently?

**Critique.** One thing, and it is a compiler error rather than a runtime surprise:

```rust
// before
for m in hits { ... }          // Match was Copy
// after
for m in &hits { ... }         // Match holds two Option<String>s
```

Everything else is additive: `Index` gains a field defaulting to empty, `to_text`/`load` accept both formats, and
`Match.provenance` is a value even when nothing was recorded — so a caller printing a result always has something to
print, and `permits_commercial_use()` still answers `None` for an unknown.

**Measured blast radius for the `Copy` loss**: **one** `Match` literal in the crate (`rank::top_k`) and no consumer
iterating by value. The decision file predicted exactly this; the prediction held.

**The honest gap**, stated in the log rather than glossed: linking a sidecar's file names to content hashes is the last
step of criterion 2, and it belongs with `indexer::build`, where the scan and the sidecar meet.

**Verdict: GREAT.**
