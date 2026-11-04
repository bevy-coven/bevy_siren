# Iteration 109 — plan: the host

## Ranked

1. **`#28` the MCP host** — chosen. Not blocked: ADR 0006 decided the layout, the issue scopes a `[[bin]]` behind `semantic`,
   and the dependency is a `leaf.rs` row. It is the gate for the wire forms of `#29`/`#30`/`#31`, whose library halves are all
   built and tested.
2. `#35` golden set — labelled real recordings: a person's work.
3. `#32`–`#34`, `#38` — `bevy_grimoire`.
4. `#39` — a music encoder and a licence audit.

**Why 1 beats 2:** it is the only open issue whose blocker was my own mistake.

## Design

**`src/semantic/server.rs`** (new, public, behind `semantic`) — the host as a library, so a test calls it with no process:

- `Host::new(index, embedder)` — holds the `Index`, a `tools::Session`, a `Toolset`, and an optional boxed embedder. **One
  `Host` is one MCP session**: the anchor is per host and lost on drop (`#42`'s pitfall, already solved by `Session`).
- `Host::handle(&mut self, line: &str) -> Option<String>` — one JSON-RPC message in, zero or one reply out. A notification
  has no reply. **This function never panics on input** and never returns a line containing a newline.
- `SPECS` — the one table: six rows (`search_music`, `set_anchor`, `find_compatible_music`, `find_compatible_sfx`,
  `explain_match`, `audition`), each with description, idempotency, parameters. `tools/list` and validation both read it.
- Result shapes are ADR 0007's, built with `serde_json::json!` — **no `Serialize` derive on a library type**, so the wire form
  is the server's and a type's fields can change without a silent wire change. Where a type has fields that must all reach
  the wire (`Weights`, `Features`), the helper **destructures it exhaustively**, so a field added later is a compile error.

**`src/bin/siren-tools.rs`** (new) — `siren-tools <library-dir> [--model <onnx> --vocab <tokenizer.json>]`: loads
`<dir>/siren-index.txt` via `indexer::read`, optionally a model and vocabulary, then reads stdin lines and writes stdout
lines. Logs go to **stderr** only (the spec: stdout carries nothing that is not a message).

**The six tools, what each does here.**

| tool | here |
|---|---|
| `set_anchor` | wired: `Session::set_anchor` |
| `find_compatible_music`, `find_compatible_sfx` | wired: `Toolset::compatible_*` |
| `explain_match` | wired: `Toolset::explain_match` |
| `search_music` | wired **when a model and vocabulary were given**; otherwise `unavailable` naming the two flags |
| `audition` | `unavailable`, as ADR 0007 states: *"with no `App` the tool returns `unavailable` rather than pretending"* |

All six are **listed always** — an agent should learn a tool exists and why it cannot run — and the listing test asserts six
names, a schema each, and `TOOLS`' idempotency on each.

**Annotations** (spec `2025-03-26+`): `readOnlyHint` true for the pure reads, false for `set_anchor` (it writes the session's
anchor) and `audition`; `idempotentHint` from `tools::TOOLS`; `destructiveHint` false; `openWorldHint` false.

## Decided and recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| transport | hand-written newline-delimited JSON-RPC, `serde_json` only | **5 crates against 57**, measured in `research.md`; synchronous work; five methods |
| feature | `serde_json` behind `semantic`, as ADR 0006 says | no new feature name; `[[bin]] required-features = ["semantic"]` |
| `bad_argument` | a tool result with `isError: true` | the model reads results, not protocol errors (`research.md`) |
| unknown tool | JSON-RPC `-32602` | the spec's own example |
| protocol versions | `2025-06-18`, `2025-03-26`, `2024-11-05` echoed if requested; else `2025-06-18` | the three revisions share these methods' shapes; the lifecycle rule is *echo if supported* |

## Fields

None on `CueTable` / `SirenSettings` / `MusicScore`. New public items: `semantic::server::{Host, Embed, SPECS, ToolSpec, Param}`.
**Defaults and their sources:** `MAX_K = 100` and `DEFAULT_K = 3` are `tools`' (ADR 0007); `find_compatible_sfx`'s default `k` is
`6` (ADR 0007: *"an effect library's useful neighbourhood is larger"*); the protocol-version list is the three revisions above.

## Tests (`tests/semantic_server.rs`, a module of `all.rs`)

1. the listing: six names, a schema each, `TOOLS`' idempotency on each, exactly one non-idempotent tool;
2. the **schema and the validator are one table**: every parameter in every `inputSchema` is accepted by `tools/call`, and a
   wrong type / out of range / missing required one is `bad_argument` before any work;
3. `initialize` echoes a supported version, answers its own for an unsupported one, declares `tools`;
4. a notification gets no reply; `ping` gets `{}`; an unknown method is `-32601`; garbage is `-32700`; an array is `-32600`;
5. **no input panics or breaks framing** — a table of hostile lines (empty, huge, deeply nested, invalid UTF-8-as-escape,
   `null` id, wrong `jsonrpc`), each answered with one line of valid JSON or nothing;
6. each wired tool against the fixture index: result shape field by field against ADR 0007's example, `asset_id` is 16 hex;
7. the **stateful** behaviour: `set_anchor` then `find_*` with no anchor argument uses it; with none set, `refused` naming
   `set_anchor`; setting twice changes nothing;
8. `search_music` with an injected embedder: returns at most `k`, default 3, `k: 0` and `k: 101` are `bad_argument` naming the
   range, an empty index is an empty list with a reason; without an embedder, `unavailable`;
9. `audition` is `unavailable`, `retryable: false`;
10. **the process**: spawn `siren-tools` on a scratch library, send `initialize`, `initialized`, `tools/list`, one call; every
    stdout line parses as JSON; stdin closing ends the process with status 0; a missing index exits non-zero **with a
    message on stderr and nothing on stdout**;
11. `cargo build` (no features) builds no binary — asserted by the manifest's `required-features`.

## Carved off

- **`audition` through a running `App`** — the cross-process hop ADR 0006 defers; it is a later issue.
- **`resources` / `prompts` capabilities** — not needed by `#28`; the server declares `tools` only.
- **Streamable HTTP** — stdio is the transport the spec says clients SHOULD support, and the one the Cauldron's own server uses.
