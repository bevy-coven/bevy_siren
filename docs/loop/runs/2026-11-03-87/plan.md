# Iteration 87 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **The binary index writer** | ADR 0004's remaining work, and the only item in the log quoted for **four iterations** without a test asserting it. A number four logs repeat and nothing checks is the definition of a magic constant. |
| 2 | `#36` indexing throughput | The other unmeasured number, scan-side. |
| 3 | `#22` spectral features | Unblocked, larger. |
| 4 | `#28` the MCP tool host | Largest user-facing win. |
| 5 | `#27` hard filters and penalties | The consumer of everything built since 77. |

**Why (1) beats (2) in one sentence:** #36 needs a corpus to be meaningful, while #87's claim is already written down and
can be **falsified by a test in this iteration** — and the test found the claim was only true of realistic data.

## The change

`Index::to_bytes` / `Index::from_bytes`, and `write`/`read` use them.

| Decision | Choice | Why |
|---|---|---|
| magic | `BSI2`, distinct from the text magic | a reader decides from the first four bytes, not the file name |
| endianness | little, throughout | one machine reads it; no exchange to design for |
| integrated loudness | **presence byte, then `f32`** | a sentinel here is the defect ADR 0003 measured |
| checksum | FNV-1a over **every preceding byte** | a truncation in any section is caught by one rule |
| read fallback | text if the magic is absent | an index written before this change still loads |
| `INDEX_FILE` | `siren-index`, one name | the bytes say which format, so the name does not have to |

No new serde field, no new threshold, no new dependency.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/index.rs` | `MAGIC_BINARY`, `IndexProblem::Bytes`, `to_bytes`, `from_bytes`, six read helpers |
| `src/semantic/indexer.rs` | `INDEX_FILE`, `write` uses bytes, `read` sniffs the magic |
| `tests/semantic_index.rs` | **5 new tests** |
| `tests/semantic_indexer.rs` | **1 new test** — a written file is binary, and an old text index still loads |

## Tests that prove it

1. `the_binary_form_round_trips_what_the_text_form_does` — all three sections, and both formats agree on the same index.
2. `five_thousand_vectors_weigh_what_adr_0004_says` — **10.28 MB** and a **≈3.1×** text ratio, now on realistic values.
3. `a_foreign_or_truncated_binary_file_is_refused` — wrong magic, half a file, three bytes.
4. `a_binary_index_from_another_model_is_refused` — the same rule the text form enforces.
5. `a_file_says_which_format_it_is` — the two magics differ.
6. `a_written_index_is_binary_and_an_old_text_index_still_loads` — compatibility, plus both refusal messages.

## Carved off

- **`f16`.** ADR 0004 measured it (5.20 MB, top-10 preserved) and declined it because `rank::top_k` compares against a
  floor; unchanged by this iteration.
- **`#36`'s throughput number.** Separate, and it needs a corpus.
