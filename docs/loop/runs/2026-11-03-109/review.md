# Iteration 109 — review

## Pass 1 — function

**Against `#28`'s acceptance criteria:**

| criterion | evidence |
|---|---|
| the binary starts and answers a tool listing | `the_binary_lists_tools_and_answers_a_search_over_stdio` (a real process, a real model graph); the **official MCP Inspector, a TypeScript client I did not write,** listed the six tools and called three of them over stdio — `evidence/after/interop.txt` |
| a malformed argument returns a structured error, not a panic | `the_schema_is_what_the_call_enforces`: every property × every violation is `bad_argument`, `retryable: true`, naming the property; `no_input_breaks_the_framing_or_panics`: 13 hostile lines |
| every tool in the listing has a schema | `the_listing_names_the_six_tools_with_a_schema_each` — six names, `tools::TOOLS`' names, a schema each |
| `cargo build` without the feature does not build the binary | `cargo build --bin siren-tools` → *"requires the features: `semantic`"*; `--bins` → *"no targets matched"* |
| the pitfall: load cost, long-lived or not | **34 ms and 20.5 MB at 10 000 assets**, then 0.02–18 ms a call; a long-lived process is the expectation (`perf.md`, ADR 0008) |

**Guards proved to fail, each by breaking the thing it guards:**

| broken | the test said |
|---|---|
| the schema's `maximum` loosened to 101 while the check stops at 100 | `search_music.k refused its own maximum` |
| one handler arm renamed | `explain_match: SPECS and its handler disagree` |
| the handler ignoring `weighting` | `under the weighting that ranked it, the explanation is the result` |
| `weights_json` written without one of `Weights`' fields (**not an experiment — it happened**) | **`E0027: pattern does not mention field compatibility`** on the first compile: a field my own grep of `Weights` had missed, caught by the exhaustive destructure |

**Critique before the verdict.** Six limits, stated rather than argued away:

1. **The protocol is hand-written, and one third-party client has exercised it.** The framing is tested against the
   specification's rules and the official Inspector connected, listed and called. Other clients (a desktop app, an IDE) have
   not. The surface is five methods and the spec's own examples, but "interoperates with the TypeScript SDK" is not "interoperates
   with every client".
2. **`search_music` has never run against the real 501 MB graph.** It is not on this machine. It has run against a 16 KB
   fixture with CLAP's interface and tokenizer, through the real `Embedder`. The first real session showed what that costs:
   the fixture embeds every phrase to cosine 1.0, so the transcript cannot show a ranking, and the ranking tests use hand-built
   vectors instead.
3. **`bad_argument` as a tool result is a judgement call.** The 2025-06-18 text lists *"invalid arguments"* under protocol
   errors and *"invalid input data"* under tool errors. ADR 0008 chose the second because a model reads results; a client that
   wanted `-32602` for a bad `k` will not get it. The Inspector surfaced `isError` correctly.
3b. **A headless server cannot be started by an agent**: no listed tool returns an id without a model. Found in the zoom
   pass, filed as `needs-decision/headless-discovery.md` because a seventh tool is a public-surface change.
4. **`find_compatible_*` is 18 ms at 10 000 assets**, over the 10 ms budget `search_ranked` meets. It is the library's
   algorithm, recorded in ADR 0008 and not changed.
5. **`audition` is listed and can never succeed from this binary.** That is what ADR 0007 says, and the cost is one wasted
   call per session at most.

**Verdict: GREAT.**

## Pass 2 — zoom

**The audio, read first because a test-only-looking change can still move a sample.** `spectrograms/footstep-grass.png`, the
0.33 s grass step: a broadband burst from 0 to about 0.027 s reaching past 10 kHz, ending on a sharp edge near −85 to −95 dBFS —
about 40 dB under the partials beside it, so the step is below audibility; three sustained low partials at roughly 0.3, 1.5 and
2.2 kHz decaying slowly; the upper partials gone by about 0.15 s; a tail near −90 dBFS. No vertical line after the attack,
no flat shelf, no hole. **All 21 wavs, `render.txt` and the refusal files are byte-identical to iteration 108's**, and the
master is **−21.5 LUFS, −7.5 dBFS true peak** — as it must be, since `src/` gained a host and touched no mix path.

**The wire, read as an agent reads it.** I read the whole of `evidence/before/session.txt` and then `after/session.txt`:

- **A real defect, line 174 against line 251.** `find_compatible_sfx` scored asset `…1002` with `acoustic` at **1.2**;
  `explain_match` of the same pair said **0.5**. The library's explanation was hard-wired to the music weights. Fixed
  (`weighting`), proved, and the after-transcript's id 13 shows 1.2 again.
- **A wording defect, `after` line 202 before the fix:** *"1 near-identical variants"*. `Tally::describe` pluralised a count of
  one. Fixed with a test.
- Numbers read as written — `0.35`, `0.85`, `1.2` — not as widened floats; a missing signal is `"raw": null, "missing": true`,
  never `0`; `weights` echoes **every** field including the nested `compatibility`.
- **Not a defect, and recorded so the next reader does not chase it:** every similarity is `1.0`, because the fixture graph
  embeds all phrases identically, and `1003` is reported as a *variant* of `1002` for the same reason.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

*A different library and a different client.* The tests run on a 4-asset hand-built index and a 20-asset one; the transcript on a
6-asset library with real-looking provenance; `hostbench` on 1 000 and 10 000 fully analysed ones; and the client for the
interop check is **not mine**. Everything held, and reading as someone who had never seen the code found two more things:

1. **A model-free server has no first step.** An agent that lists the tools sees `set_anchor(asset_id)`, and nothing says where
   an `asset_id` comes from unless a model is loaded. It answers correctly and cannot be started. This is the finding I would
   not have made from the tests, because every test supplies the ids. It is a *design* gap in ADR 0007's six tools, so it is
   `needs-decision/headless-discovery.md` with four options and a recommendation, not a patch.
2. **My own reasoning was wrong in the way this loop keeps being wrong.** The previous seven iterations treated `#28` as blocked
   on a decision that ADR 0006 had already made, and I only found out because I read an ADR to name an *unrelated*
   follow-up. The decision file I wrote recommended the option the ADR had rejected. The remedy is not a better rule; it is
   reading the accepted ADRs **before** filing a question about the layout — which is what this iteration did first, and what
   made it a build rather than a seventh drift guard.

Read as a maintainer: one table (`SPECS`) is the listing, the schema and the validation; a tool needs a row and a handler and
a test fails until it has both; a field added to `Weights` is a compile error in the wire form; the host is a library, so
nothing below the binary needs a pipe to test. Read as a client: initialise, list, call; a bad call says what to fix and
whether fixing it will help.

**Verdict: GREAT.**
