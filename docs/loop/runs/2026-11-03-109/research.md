# Iteration 109 — research: the MCP host (`#28`)

## The item

`#28` — *"a server binary behind the `semantic` feature, listing the tools `S-07` names; argument validation that rejects a
bad call with a typed error before any work happens; one tool wired end to end; a test that lists the tools and asserts the
count and names."* Acceptance: the binary answers a listing; a malformed argument returns a structured error, not a panic;
every tool in the listing has a schema; `cargo build` without the feature does not build the binary.

## What was already decided, and what I got wrong about it

| Source | What it says |
|---|---|
| ADR 0006 (**accepted**, 2026-11-03) | *"One library crate, one `[[bin]]` behind the `semantic` feature … `src/bin/siren-tools.rs` … A workspace was considered and rejected."* Its cost table: the binary's dependencies are *"argued for … like any other"* in a `tests/leaf.rs` row |
| `#28`'s own scope | *"A server binary behind the `semantic` feature"* |
| ADR 0007 (**accepted**) | the six tools, their parameters, the result and error shapes, `k`'s bound, the idempotency table |
| `needs-decision/README.md` | *"only the public API, the message types and the definitions-file schema come here"* |

`needs-decision/mcp-tool-surface.md` (iteration 102) re-asked the layout question, **recommended the workspace ADR 0006 had
rejected**, and listed `#28`–`#31` as blocked. A feature-gated binary is none of the three things the README reserves, and its
dependency is a `leaf.rs` row the ADR says to argue. It was never blocked. The file now carries a banner (iteration 108).

## Measured, before writing code

| | |
|---|---|
| unique crates, normal edges, `--features semantic` | **177** (482 lines of `cargo tree`) |
| unique crates, normal edges, default | **149** (413 lines) |
| `serde_json` in that normal graph | **absent** — it appears only through the dev-dependency umbrella (`bevy_gltf`) |
| `serde_json` alone, as a probe | **6** crates incl. the probe: `itoa`, `memchr`, `serde_core`, `serde_json`, `zmij` |
| `rmcp = "=3.3.0"` (`server`, `macros`, `transport-io`) + `tokio` + `schemars` + `serde_json`, as a probe | **58** crates incl. the probe: a full async stack (`tokio` and nine `futures` crates), `chrono`, `uuid`, `getrandom`, `libc`, and the proc-macro tree (`syn`, `darling`, `quote`, `schemars_derive`) |

## The one real decision: hand-written stdio, or the sibling's `rmcp`

`bevy_grimoire`'s `bevy_cauldron_mcp` uses `rmcp = "=3.3.0"` with `tokio`, `schemars` and `serde_json`
(`crates/bevy_cauldron_mcp/Cargo.toml`), so that is **the org's existing convention** and I looked at it before choosing
otherwise. Three reasons it does not carry over:

1. **Cost: 57 crates against 5**, measured above. `bevy_cauldron_mcp` is an application that already links Bevy; this crate's
   `semantic` feature is a *library* feature a game turns on, and `tests/leaf.rs` exists so that stays small.
2. **The work is synchronous.** `ort` inference, the index scan and the ranking are blocking calls on one request at a time;
   an async runtime would be a thread pool wrapped around a loop that reads a line, answers, and writes a line.
3. **The protocol surface needed is five methods** — `initialize`, `notifications/initialized`, `ping`, `tools/list`,
   `tools/call` — each a few lines of JSON, and `rmcp`'s macros (`#[tool]`) would put the schema in attributes where ADR 0007's
   *"the schema is a description of the function, not a second API"* wants it in one table the tests can enumerate.

What this costs: **I own the protocol framing**, so it is tested against the spec's own examples (below) and against a real
client's handshake. A third reason to keep it small.

## Sources, and the numbers they report

| Source | What it reports |
|---|---|
| MCP spec `2025-06-18` `basic/transports` (read 2026-11-03) | **stdio**: messages are newline-delimited and **MUST NOT contain embedded newlines**; the server **MAY** log to stderr; it **MUST NOT** write anything to stdout that is not a valid MCP message |
| MCP spec `basic/lifecycle` | `initialize` → result `{protocolVersion, capabilities, serverInfo, instructions?}` → client's `notifications/initialized`. **Version negotiation**: if the server supports the requested version it **MUST** answer the same one, otherwise *"another protocol version it supports"* (SHOULD be its latest). Clients **SHOULD NOT** send requests before the `initialize` response except `ping` |
| MCP spec `server/tools` | `tools/list` → `{tools: [{name, title?, description, inputSchema, outputSchema?, annotations?}], nextCursor?}`; `tools/call` → `{content: [...], structuredContent?, isError}`. *"A tool that returns structured content SHOULD also return the serialized JSON in a TextContent block."* Servers **MUST** declare the `tools` capability and **MUST** validate all tool inputs |
| the same page, *Error Handling* | **two mechanisms**: JSON-RPC **protocol errors** (the example is `-32602`, *"Unknown tool"*) and **tool execution errors**, reported in the result with `isError: true` |
| JSON-RPC 2.0 | `-32700` parse error, `-32600` invalid request, `-32601` method not found, `-32602` invalid params, `-32603` internal error; a notification has no `id` and gets **no** response |
| *The Hitchhiker's Guide to Agentic AI* (arXiv `10.48550/arXiv.2606.24937`) §22.5 | *"the LLM uses the name and description to decide which tool to call and when"*; annotations `readOnlyHint`, `destructiveHint`, `idempotentHint` (*"Safe to retry on failure"*), `openWorldHint`, added in the `2025-03-26` revision |
| *MCP at First Glance* (ACM, `10.1145/3814959`) §3.2 | MCP decouples tools from frameworks: tools are hosted in standalone servers and reached through a standard client |
| `bevy_grimoire/crates/bevy_cauldron_mcp/tests/an_agent_calls_the_server.rs` | a real client's order: `initialize` with `"protocolVersion": "2025-06-18"`, `notifications/initialized`, `tools/list`, `tools/call` with `{name, arguments}` |

## The technique

**One table drives the listing, the validation and the schema.** Each tool is a row: name, description, idempotency (from
`tools::TOOLS`), and its parameters — name, kind (string / bounded integer), required, default, description. `tools/list`
renders each row to an `inputSchema`; `tools/call` validates the arguments against **the same row** before calling anything.
The schema and the validator cannot disagree because there is one of them. A tool added without a row is not callable; a row
without a function is a failing test.

**`bad_argument` is a tool execution error, not a protocol error.** The spec lists *"invalid arguments"* under protocol errors
and *"invalid input data"* under tool errors; the deciding fact is who reads it. ADR 0007's whole case is that *"an agent that
cannot parse an error cannot recover"*, and a model sees a tool result, while a JSON-RPC error is addressed to the client
program. So a `k` of 0, a malformed `asset_id`, a missing required parameter and a wrong type are all `isError: true` results
carrying ADR 0007's `{error: {code, message, retryable}}`. **Protocol errors stay for the protocol**: unparseable JSON
(`-32700`), a message that is not a request (`-32600`), an unknown method (`-32601`), an unknown *tool* (`-32602`, the spec's
own example), and a `tools/call` with no `name` (`-32602`).

## What good looks like

An agent that connects, lists, calls `set_anchor`, then `find_compatible_sfx`, gets ADR 0007's shapes; calls with `k: 0` and
reads *"k must be between 1 and 100, and 0 is not"* with `retryable: true`; calls `find_compatible_music` with no anchor and
reads *how to set one* with `retryable: false`; and **never sees a panic, a hang, or a line on stdout that is not JSON.**
