# Decision: how an agent finds its first `asset_id` when the server has no model

## The question

`siren-tools` started **without** `--model` serves five tools, and four of them need an `asset_id` the caller already holds:
`set_anchor`, `find_compatible_music`, `find_compatible_sfx` (without an anchor) and `explain_match`. **The only tool that
returns ids from nothing is `search_music`, and it needs the model.** So a headless server answers every call correctly and
cannot be *started* by an agent: it has no way to learn one id.

Found by reading `evidence/after/interop.txt` as a client that had never seen the library — the official MCP Inspector's
`tools/list` shows the tools, and no listed tool says where an id comes from without a model. A person who already holds ids
(from the editor's panel, or from an earlier `search_music`) is unaffected.

This is a change to the **public tool surface** (ADR 0007's six tools are a semver surface), so the loop files it rather than
deciding it.

## Options

1. **A seventh tool, `list_assets(category?, k = 6)`.** Returns at most `k` assets — id, provenance, and the features the
   index holds — optionally filtered by the same `category` text `find_compatible_sfx` takes, in a **total order** (ascending
   id) so two calls agree. `k` is a bound with the same ceiling as every other tool, so it cannot ask for a library dump.
   Cost: a seventh schema and a name that can never change; ADR 0007 argued *"six tools, not sixteen"*, and seven is still
   small. Benefit: a model-free session has a first step, and the Cauldron's panel gets its browse list from the same call.
2. **Let `search_music` work without a model, by matching the description against provenance** (`source`, `licence`, `author`).
   Cost: it changes what the tool means — *"describe a sound in words"* becomes *"keywords against file metadata"* — and a
   result from it would carry `similarity: null`, a different shape from the model-backed answer. Benefit: no new tool.
3. **Leave it.** The headless server is for callers that already hold ids, and the doc says so. Cost: an agent pointed at a
   model-free server stalls with a correct, unhelpful `refused`.
4. **Return the first assets in the `unavailable` message of `search_music`.** Cost: a tool result that smuggles a second
   tool's output, so the schema no longer describes what comes back.

## Recommendation

**Option 1**, with `k` bounded and the order total. It is the smallest change that gives a model-free session a first step,
it keeps every tool's meaning exactly what ADR 0007 says, and *"every tool's body is one call to a public function"* holds:
`list_assets` is one pass over `Index::records`, which a panel can call directly with no protocol.

If the owner would rather keep six, option 3 is defensible and costs one sentence in the guide; **not** option 2.

## What is blocked

Nothing. Headless use works for any caller that holds ids, and `search_music` works with a model. This file only records that
the model-free path has no entry point of its own.
