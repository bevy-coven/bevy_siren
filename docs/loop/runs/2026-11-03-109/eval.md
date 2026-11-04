# Iteration 109 — eval

## Baseline before anything

Nothing red — iteration 108's end state: `cargo test` 199 + 6 doc; `--features serde` 208; `--features mixer` 209;
`--features semantic` 109 lib + **436** + 6 doc; `--all-features` **460**; clippy, fmt and docs clean; examples build.

## After the change

| run | result |
|---|---|
| `cargo test` | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 doc |
| `cargo test --features mixer` | 209 + 6 doc |
| `cargo test --features semantic` | 109 lib + **478** + 6 doc (**+42**: 41 in `semantic_server`, 1 in `semantic_tools`) |
| `cargo test --all-features` | 109 lib + **502** + 6 doc |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean — after fixing one `useless_vec` of my own in the new test file |
| `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |
| `cargo build`, `cargo build --examples --features semantic` | ok |
| `cargo build --bin siren-tools` (no features) | **`error: target siren-tools … requires the features: semantic`** |
| `cargo build --bins` (no features) | *"target filter `bins` specified, but no targets matched"* |
| `cargo tree --edges normal`, default | **413 lines, 149 unique crates — unchanged** |
| `cargo tree --edges normal --features semantic` | 482 → **487** lines, 177 → **181** unique crates (`serde_json` and three others — one of its four dependencies was already in the graph) |

The two clippy/fmt findings were mine and are fixed; nothing was red before this iteration touched it.

## The cue cases

All 33 of `cues.yaml` green, unchanged. **All 21 rendered wavs, and `render.txt` and the refusal files, are byte-identical to
iteration 108's** (`cmp`, no differences) and the master is **−21.5 LUFS, −7.5 dBFS true peak**: this iteration adds a tool
host and changes no sample.

## The cases

| Case | Green/red | Observation |
|---|---|---|
| `#28`: the binary starts and answers a tool listing | **green** | `the_binary_lists_tools_and_answers_a_search_over_stdio`; and the **official MCP Inspector (TypeScript SDK)** listed the six tools over stdio against the real binary — `evidence/after/interop.txt` |
| `#28`: a malformed argument is a structured error, not a panic | **green** | `the_schema_is_what_the_call_enforces` (every property × every violation → `bad_argument`, `retryable: true`, naming the property); `no_input_breaks_the_framing_or_panics` (13 hostile lines: 10 000-deep nesting, a 3 MB line, a lone surrogate, a BOM…) |
| `#28`: every tool in the listing has a schema | **green** | `the_listing_names_the_six_tools_with_a_schema_each` — six names, `TOOLS`' names, a schema each |
| `#28`: `cargo build` without the feature builds no binary | **green** | the two cargo outputs above, and the manifest assertion |
| `#28`'s pitfall: load cost, and whether a long-lived server is expected | **answered** | **34 ms and 20.5 MB at 10 000 assets**, then 0.02 ms to 18 ms a call: `perf.md`, ADR 0008 |
| the schema is what the call enforces | **proved** | tightening the check by one (schema says 101, check says 100) → `search_music.k refused its own maximum` |
| a tool in `SPECS` has a handler | **proved** | renaming one handler arm → `explain_match: SPECS and its handler disagree` |
| every field of `Weights` reaches the wire | **fired for real** | the exhaustive destructure failed to compile on first build: **`E0027 … missing field compatibility`** — a field my own grep had missed |
| `explain_match` agrees with the search it explains | **RED, found in the real session, fixed** | `find_compatible_sfx` scored a pair with `acoustic` at **1.2**; `explain_match` of the same pair said **0.5** (`evidence/before/session.txt` lines 174 and 251). Now `weighting` selects the effects weights, and the explanation **is** the result — proved by ignoring the argument and watching the test fail |
| `bad_argument` is a tool result a model reads | **green** | `isError: true` with ADR 0007's shape, in-process and through the Inspector |
| a notification is never answered or executed | **green** | `a_notification_is_never_answered_or_run`: a `tools/call` sent with no `id` set no anchor |
| the framing survives an over-long line | **green** | drained through its newline, the next message still reads |
| 478 semantic / 502 all-features | **green** | |

## What the first real session found

Reading `evidence/before/session.txt` — the transcript of the real binary, not a test — showed three things no test had:

1. **`explain_match` used the music weights for an effect.** Individually right numbers that did not add up to the search's
   score. Fixed (above), with a library-level test as well.
2. **The fixture model embeds every phrase to cosine 1.0**, so the transcript shows every similarity as `1.0` and near-duplicate
   suppression at work on assets that are not duplicates. That is the 16 KB fixture graph, not CLAP; the transcript says so on
   its second line. It is the reason the ranking assertions in the tests use hand-built vectors.
3. **`indexer::INDEX_FILE` is `siren-index`**, not the `siren-index.txt` two doc comments said — found because the process test
   for a missing index asserted the documented name and failed.

## The item

`#28`: `semantic::server` (the host, as a library), `src/bin/siren-tools.rs` (the loop), `serde_json` behind `semantic` with its
`leaf.rs` row, six tools listed, five wired and `audition` `unavailable` as ADR 0007 says, `examples/hostbench`, ADR 0008.
