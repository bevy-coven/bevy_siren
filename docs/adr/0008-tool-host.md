# 0008 — The tool host: a hand-written stdio transport, and what a bad argument is on the wire

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#28](https://github.com/bevy-coven/bevy_siren/issues/28)
- **Supersedes:** nothing. **Depends on:** [0006](0006-plugin-and-tool-server.md), [0007](0007-tool-surface.md)

## The question

[ADR 0006](0006-plugin-and-tool-server.md) puts the tool server in this crate as one `[[bin]]` behind `semantic`, and
[ADR 0007](0007-tool-surface.md) fixes the six tools, their parameters and their result shapes. Neither says **how the bytes
move**, **which dependency carries them**, or **which of MCP's two error mechanisms a bad argument uses**. `#28` is where
those get answered, and each answer is a semver surface for an agent.

## The decision

**A hand-written newline-delimited JSON-RPC 2.0 loop over stdio, with `serde_json` as the only new dependency.** The protocol
and the tools live in `semantic::server`, so a test drives them with no process; `src/bin/siren-tools.rs` is a loop around it.

### Why not `rmcp`

`bevy_grimoire`'s own server uses `rmcp = "=3.3.0"` with `tokio` and `schemars`, so it is the org's existing convention and was
considered first. Measured with two probe crates (`cargo tree --edges normal`, unique crates including the probe):

| stack | crates |
|---|---|
| `serde_json` alone | **6** — `itoa`, `memchr`, `serde_core`, `serde_json`, `zmij`, and the probe itself |
| `rmcp` (`server`, `macros`, `transport-io`) + `tokio` + `schemars` + `serde_json` | **58** — a full async runtime (`tokio` and nine `futures` crates), `chrono`, `uuid`, `getrandom`, `libc`, and the proc-macro tree |

The server there is an application that already links Bevy. This is a **library feature** a game turns on, and the work it
wraps — ONNX inference, an index scan — is synchronous and one request at a time, so the runtime would be a thread pool around
a loop that reads a line and writes a line. The protocol needed is **five methods**. The cost of writing it ourselves is that
we own the framing, which is why `tests/semantic_server.rs` is built from the specification's own rules and drives hostile
input at it.

### A bad argument is a tool result, not a protocol error

The MCP specification names two mechanisms: JSON-RPC errors for protocol problems, and a result with `isError: true` for a tool
that ran and failed. It lists *"invalid arguments"* under the first and *"invalid input data"* under the second, so the
specification alone does not decide. **Who reads it does.** A model reads tool results; a JSON-RPC error is addressed to the
client program, which may never show it. ADR 0007's case is that *"an agent that cannot parse an error cannot recover"*, so:

| | mechanism |
|---|---|
| `k: 0`, a malformed `asset_id`, a missing required argument, a wrong type, an unknown argument | **tool result**, `isError: true`, ADR 0007's `{"error": {"code": "bad_argument", "message", "retryable": true}}` |
| a state refusal (`NoAnchor`, `AnchorNotIndexed`), `unavailable`, `internal` | the same shape, `retryable: false` |
| unparseable JSON, a message that is not a request, an unknown method, an unknown **tool**, a `tools/call` with no `name` | **JSON-RPC error** (`-32700`, `-32600`, `-32601`, `-32602`) |

An unknown tool is `-32602` because it is the specification's own example for it.

### Where ADR 0007 was silent

Each of these was decided here rather than left to the code, because each is on the wire.

| decision | choice | why |
|---|---|---|
| a list result's `summary` | **added**: `Answer::describe()`, a sentence | `#29`'s fourth criterion is *"an empty library returns an empty list **with a reason string**"*, and the counts in `tally` are not a reason. The library already builds the sentence; the wire form omits nothing the type has |
| `search_music` and the session's anchor | **it does not read it.** An `anchor` argument is the only way to judge a search against an asset | ADR 0007 lists it as *"a pure read; the same description and index give the same answer"*. Reading session state would make the same call answer differently after `set_anchor` and make that statement false |
| a headless host's `search_music` | listed, and `unavailable` with the two flags that fix it | an agent should learn a tool exists and why it cannot run — the reason ADR 0007 gives for `audition` |
| `audition` | listed, `unavailable` | ADR 0007: *"with no `App` the tool returns `unavailable` rather than pretending"* |
| `explain_match`'s result | `{"result": <found>, "weights": <the music weights>}` | a `Found` is the one-result shape, and `weights` is echoed on every answer so it can be reproduced |
| `set_anchor`'s result | `{"anchor": {asset_id, dimension, features, provenance}}` | *"the anchor and what it knows about it"* (`#42`). The 512-float embedding is **not** sent: it is noise to an agent and the dimension says what it is |
| a number | the shortest decimal that reads back as the same `f32` | `0.683`, not the `0.6830000281333923` that widening to `f64` prints |
| `k: 3.0` | accepted as `3` | JSON Schema 2020-12 counts it as an integer, and the schema the server advertises says `integer` |
| `explain_match`'s `weighting` | **added**: `"music"` (default) or `"effects"`, echoed in the result as `weighting` | the first real session showed `find_compatible_sfx` scoring a pair with `acoustic` at 1.2 and `explain_match` of the same pair saying 0.5, because the library's explanation was hard-wired to the music weights. An explanation under a different weighting than the search it explains has numbers that are individually right and do not add up. Adding an optional parameter with a default is compatible (ADR 0007's own rule); `Toolset::explain_match` keeps its signature and `explain_match_with` takes the weights |
| an unknown argument | refused, naming what the tool takes | `additionalProperties: false` in the schema; the crate's own rule is that an unknown field is refused, not half-read |
| every problem in a call | reported together | an agent with two mistakes fixes both in one retry |
| `inputSchema` and the check | **one table** (`semantic::server::SPECS`) | the schema a client reads and the validation its call meets cannot disagree, because there is one of them |

## The wire, from the specification

Revision `2025-06-18` (`basic/transports`, `basic/lifecycle`, `server/tools`); the host also answers `2025-03-26` and
`2024-11-05`, which share these methods' shapes.

- **stdio**: one message per line, **no embedded newlines**; stdout carries nothing that is not a message; logs go to stderr.
  The host's replies are compact JSON, which cannot contain a raw newline, and a line longer than `MAX_LINE_BYTES` (1 MiB,
  **design default**) is refused and **drained through its newline** so the stream stays aligned.
- **`initialize`**: answered with the requested version if supported, else the latest. A notification is **never answered and
  never executed** — a `tools/call` sent with no `id` would otherwise run with no way to report what happened.
- **Results**: `structuredContent` and, for older clients, the same JSON serialised in a text block.
- **No `outputSchema`**: the specification makes a server that declares one **MUST** conform, and an error result is a
  different shape. The shapes are ADR 0007's.

## State, scope and cost

The anchor belongs to the [`Host`](../../src/semantic/server.rs): one connection, one host, lost when it drops. **The index is
read once at start-up and held in memory; a query is a scan, not a load**, so a long-lived process is the expectation — the
stdio transport launches one server per client session. Measured by `examples/hostbench`, release build, this Mac, fully
analysed libraries:

| assets | index file | read | `Host::new` | `find_compatible_*` p50 | `search_music` p50 (no encode) | `set_anchor`, `explain_match` |
|---|---|---|---|---|---|---|
| 1 000 | 2.05 MB | 3.4 ms | 0.2 ms | 0.70 ms | 0.55 ms | 0.02 ms |
| 10 000 | 20.51 MB | 34.2 ms | 1.2 ms | **18.2 ms** | 5.3 ms | 0.02 ms |

**Not measured: loading the 501 MB ONNX graph, and the text encode** — the graph is not on this machine. A host started
without `--model` pays neither.

`find_compatible_*` is 3.5× the cost of `search_music` at 10 000 assets because it scores **every** candidate with the
ensemble before truncating to `k` (the hard rules and the weights reorder the result, so it cannot rank by cosine first the way
`search_ranked` does). That is the library's behaviour, recorded here rather than changed.

## Cost to a consumer

| | |
|---|---|
| default build | unchanged; `cargo build` builds no binary (`required-features = ["semantic"]`) |
| a user who enables `semantic` | one more dependency, `serde_json`, and a `siren-tools` binary. Measured with `cargo tree --edges normal`: **482 → 487 lines, 177 → 181 unique crates**; without `semantic` it is unchanged at 413 lines and 149 crates |
| a panel author | unchanged: the panel calls the library, and `semantic::server` is not on its path |
| a tool's parameters | a semver surface, as ADR 0007 says; the schema in `tools/list` is the contract |

## What would reverse this

A need for **streaming results, progress notifications, resources or prompts** — then `rmcp`'s transport earns its 52 extra
crates. Nothing in the target workflow needs any of them: a search is one request and one answer.
