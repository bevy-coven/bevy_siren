> **Superseded by [ADR 0006](../../adr/0006-plugin-and-tool-server.md) — and this file should never have been filed.**
> ADR 0006 was *accepted* on 2026-11-03 and already decides what this file asks: *"One library crate, one `[[bin]]` behind
> the `semantic` feature … A workspace was considered and rejected."* That is **Option A below, and the opposite of the
> recommendation written here.** Issue `#28`'s own scope says the same (*"a server binary behind the `semantic` feature"*).
> I filed this without reading the ADR's decision, recommended the option it had rejected, and then treated `#28`–`#31` as
> blocked on it for several iterations.
>
> What actually remains is one dependency row. ADR 0006's cost table says the binary's dependencies are *"argued for … like
> any other"* in a `tests/leaf.rs` row, and `needs-decision/README.md` says only *"the public API, the message types and the
> definitions-file schema come here"* — a feature-gated binary is none of the three. **Iteration 109 builds the server.** The
> file stays as the record of the mistake; nothing below it is a pending decision.

# Decision: where the MCP tool surface lives, and what it costs the crate

`#28` asks for "a server binary behind the `semantic` feature, listing the tools `#8` names". The crate today has **no
JSON dependency**: `tests/leaf.rs` enforces a closed list (`bevy`, `serde`, `ron`, `rodio`, and the semantic feature's
`ort`, `ebur128`, `rustfft`, `tokenizers`, `symphonia`, `rubato`). An MCP server needs a JSON codec and a transport.

This is a **dependency-list change and a new binary target**, which the build loop's rules put here rather than deciding
unilaterally. The three tool families themselves — `T-03`, `T-04`, `T-05` — are already built as library calls in
`src/semantic/tools.rs` and `search::search_ranked`, so whatever this decision says, the work they wrap exists.

## Option A — a `[[bin]]` in this crate, `serde_json` behind `semantic`

```toml
[[bin]]
name = "bevy_siren_mcp"
required-features = ["semantic"]

[dependencies]
serde_json = { version = "1", optional = true }
```

- **Cost to the crate**: one dependency, plus the `leaf.rs` row that argues for it. It stays a leaf: `serde_json` names
  no engine, and the binary is behind the feature so `cargo build` without it builds nothing new.
- **Cost to a consumer**: none unless they enable `semantic`; a game never links a server.
- **Risk**: a library crate that also ships a binary is a slightly odd shape for crates.io, and the binary's feature set
  is now part of the semver surface for the *library's* `semantic` feature.

## Option B — a separate crate in a workspace

```
bevy_siren/            # the library, unchanged
bevy_siren_mcp/        # the server, depends on bevy_siren + serde_json + an MCP transport
```

- **Cost to the crate**: a workspace root, a second `Cargo.toml`, a second CI target. The library's dependency list and
  `leaf.rs` stay exactly as they are — which is the property `leaf.rs` exists to protect.
- **Cost to a consumer**: none; and a game that never enables the feature never even sees the workspace.
- **Risk**: `cargo publish` for two crates, versioned together, and the release procedure in the build loop's step 7
  assumes one. The two-crate release is a real ongoing cost.

## Option C — no server in this repository

`#28`'s host lives in `bevy_grimoire` (which already has an MCP server for the editor) or in a third repository. This
crate ships the **functions** and a documented signature for each; the host is somebody else's binary.

- **Cost to the crate**: nothing — no new dependency, no binary, no workspace.
- **Cost to a consumer**: an agent can only reach these tools through whatever host adopts them; the crate cannot ship a
  working `search_music` over MCP on its own, which is what `#29`'s acceptance criteria assume.
- **Risk**: the tool result shapes are then defined where they are consumed rather than where the data is, which is the
  coupling `#8`'s ADR exists to prevent.

## Recommendation

**Option B**, for one reason: `tests/leaf.rs` is a load-bearing promise — *"this crate is sound any game can take"* — and
Option A makes the crate's own `semantic` feature depend on a JSON codec that has nothing to do with selecting audio.
The workspace keeps the library's dependency list exactly what the leaf test pins, and the server can take whatever
transport an MCP implementation needs without that argument touching the audio crate.

Option A is defensible and simpler; if the owner would rather have one crate, the `leaf.rs` row to write is *"serde_json:
the tool surface's wire format, behind `semantic`, and it names no engine"*.

## What is blocked until this is decided

- `#28` (the host), `#29` (`search_music`), `#30` (`audition`), `#31` (structured errors and pagination).
- `#33` (the editor auditions through the crate's buses) is **not** blocked — it is in-process.

## What is not blocked, and is already built

`set_anchor`, `find_compatible_music`, `find_compatible_sfx`, `explain_match`, and `search_ranked` are library calls with
tests, so an editor panel or a future host has something to call today.
