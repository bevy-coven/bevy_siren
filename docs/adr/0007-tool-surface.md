# 0007 — The tool surface: names, parameters, results, and how a panel calls the same thing

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#8](https://github.com/bevy-coven/bevy_siren/issues/8)
- **Supersedes:** nothing. **Depends on:** [0006](0006-plugin-and-tool-server.md), [0004](0004-vector-store.md)

## The question

What is the exact tool surface, and what does it return? A tool schema is a public API: once an agent calls
`search_music`, changing its parameters is a breaking change in the same way a Rust signature is. And the schema must be
consumable twice — by an agent over MCP and by `bevy_grimoire`'s panel with no second protocol.

## The decision

**Option (c), a hybrid: task-shaped tools over primitives, and every tool is a thin wrapper over a function that already
exists in this crate.**

Option (a) — one tool per Rust function — was rejected because it makes an agent's cheapest workflow three round trips
(*embed, then rank, then explain*) for something the crate can answer in one call. Option (b) — one task-shaped
`describe_and_pick` — was rejected because it removes the ability to iterate: an agent that gets a bad top result needs
to set an anchor and ask a different question without starting over.

**The rule that keeps a panel protocol-free: every tool's body is one call to a public function.** The MCP server
serialises arguments, calls the function, and serialises the result. The panel calls the function. There is one
implementation, and the schema is a description of it rather than a second API.

## The tools

Six, and each names the Rust call it wraps.

| tool | wraps | parameters |
|---|---|---|
| `search_music` | `search::search_ranked` | `description: string`, `k: integer = 3`, `anchor: string?` |
| `set_anchor` | `tools::Session::set_anchor` | `asset_id: string` |
| `find_compatible_music` | `tools::Toolset::compatible_music` | `anchor: string?`, `k: integer = 3` |
| `find_compatible_sfx` | `tools::Toolset::compatible_sfx` | `anchor: string?`, `category: string?`, `k: integer = 6` |
| `explain_match` | `tools::Toolset::explain_match` | `asset_id: string`, `anchor: string` |
| `audition` | a `Play` message | `asset_id: string`, `bus: string = "Effects"` |

**`anchor` omitted on the two `find_*` tools means the session's anchor.** That is the only stateful behaviour, and it is
scoped to one `Session` object per client — [ADR 0006](0006-plugin-and-tool-server.md)'s "no second copy of state" applies
to it: the server holds the session, and nothing in the `App` mirrors it.

**`k`'s defaults differ on purpose.** `search_music` is 3 because that is the stated workflow (*describe, see three,
pick one*). `find_compatible_sfx` is 6 because an effect library's useful neighbourhood is larger — twenty footsteps that
belong together are a set, not a shortlist.

**`asset_id` is the content hash, as 16 lowercase hex characters.** Not a path: a path is absolute to the machine that
indexed it, which makes a result unusable by a panel or another tool. That is `#29`'s own technical note.

## Result shapes

Every result is one object, and the fields are the ones the Rust type already has. `score` and `breakdown` are kept
**separate** because they answer different questions — on what scale the model matched, versus why it ranked there once
the project's weights had their say.

```json
{
  "results": [
    {
      "asset_id": "078e905a43bf405d",
      "score": 0.842,
      "similarity": 0.683,
      "breakdown": {
        "parts": [
          { "signal": "semantic", "raw": 0.683, "weight": 1.0, "contribution": 0.683, "missing": false },
          { "signal": "acoustic", "raw": -0.25, "weight": 0.5, "contribution": -0.125, "missing": false },
          { "signal": "tempo", "raw": null, "weight": 0.35, "contribution": 0.0, "missing": true },
          { "signal": "key", "raw": null, "weight": 0.35, "contribution": 0.0, "missing": true }
        ]
      },
      "explain": "semantic carried it (+0.683 from 0.683 raw at weight 1); acoustic counted against it (-0.125 from -0.25 raw); no value for tempo, key",
      "provenance": { "licence": "CC0-1.0", "source": "pack:Footsteps", "author": null }
    }
  ],
  "tally": { "returned": 1, "excluded": 0, "below_floor": 0, "variants": [], "wrong_category": 0 },
  "weights": { "semantic": 1.0, "acoustic": 0.5, "tempo": 0.35, "key": 0.35, "...": "the rest as authored" }
}
```

**A missing signal is `null` with `"missing": true`, not `0`.** A zero contribution from a signal that had nothing to say
and a zero from one that was consulted and disagreed are different facts, and an agent that cannot tell them apart will
draw the wrong conclusion.

**`weights` is echoed on every answer** so a result can be reproduced. An answer whose weights are not recorded is an
answer about a configuration nobody can name.

## The error shape

One shape, and the distinction the issue asks for is the top-level field:

```json
{ "error": { "code": "refused", "message": "no anchor is set. Call `set_anchor(asset_id)` first, or pass one explicitly", "retryable": false } }
```

| `code` | meaning | `retryable` | example |
|---|---|---|---|
| `bad_argument` | the call could not be understood | **true** — fix the argument and call again | `k: 0`, a malformed `asset_id`, `k: 10001` |
| `refused` | the call was understood and cannot be answered in this state | **false** — the *state* must change | `NoAnchor`, `AnchorNotIndexed` |
| `unavailable` | the capability is not present | **false** | `audition` with no running `App` |
| `internal` | a bug | **false** | an embedder producing a short batch |

**`bad_argument` versus `refused` is the distinction that matters.** A bad argument is the agent's mistake and retrying
after a fix is exactly right; a refusal is a fact about the library or the session, and retrying unchanged will fail
identically. Conflating them is how an agent enters a retry loop.

**A refusal to *play* is a different thing from a refusal to *search*.** `audition` returns the crate's own `Refused`
verbatim — `UnknownCue`, `NoSounds`, `MissingSound`, `UnknownBus`, `VoiceLimit`, `SpatiallyMute` — because that is the
answer the plugin gave, and rewriting it into `refused` would lose which of the six it was. Those carry
`"code": "play_refused"` with the crate's variant name in `"reason"`.

## Pagination and idempotency

**Pagination: none, and that is the decision.** `k` is a bound, not a page — the tools return at most `k` results and `k`
has a ceiling, `tools::MAX_K = 100` (**design default**; larger than any workflow needs, small enough that a hostile `k`
cannot ask for a library dump), so no call can return an unbounded list. A cursor over a ranking is meaningless anyway: the
ranking is total and the same query returns the same order, so *"the next page"* is *"a larger `k`"*, which is one call
rather than a sequence.

`k == 0` or `k > MAX_K` is `bad_argument`, with the valid range in the message, and it is refused **before** any work
happens — `tools::check_k` is the one place that decides.

| tool | idempotent? | why |
|---|---|---|
| `search_music` | **yes** | a pure read; the same description and index give the same answer field for field |
| `set_anchor` | **yes** | setting the same anchor twice stores the same hash; tested as `setting_the_same_anchor_twice_changes_nothing` |
| `find_compatible_music` | **yes** | a pure read against the session's anchor |
| `find_compatible_sfx` | **yes** | the same |
| `explain_match` | **yes** | two hash lookups and one cosine |
| `audition` | **NO** | **it starts a voice.** Retrying it plays the sound twice, which is a different result and, on a `VoiceLimit`, a different refusal |

**`audition` is the only non-idempotent tool**, and saying so is why the field exists. An agent retrying a timeout on it
would double-play the asset.

## How a panel calls the same surface

Not prose — the panel calls the functions, and the JSON above is a serialisation of their results:

```rust
use bevy_siren::semantic::tools::{Session, Toolset};
use bevy_siren::semantic::search::{Ranked, search_ranked};

// The same call `search_music` makes, with no JSON in sight.
let request = Ranked { embedding: &embedding, anchor, features: &features, k: 3, floor, weights: &weights };
let answer = search_ranked(&index, &stamp, &query, &request)?;

// The same call `set_anchor` and `find_compatible_sfx` make.
let mut session = Session::new();
let anchor = session.set_anchor(&index, chosen)?;
let effects = Toolset::default().compatible_sfx(&index, &session, None, Some("footsteps"), 6)?;

// And `audition` is a message the plugin already drains — never a second play path. The bus is the cue's, not the
// message's: a cue names its bus, so auditioning "through the Effects bus" means the cue's own bus is Effects.
commands.write(Play::cue("footstep.stone"));
```

**No serialisation, no server, no protocol.** That is criterion 4, and it is why the tools are library calls first:
`src/semantic/tools.rs` and `src/semantic/search.rs` were built before any host existed.

## Consequences

- **A tool's parameters are a semver surface.** Adding an optional parameter with a default is compatible; renaming one
  or changing a default is not.
- **The server cannot be built in this crate yet.** It needs a JSON codec, and `tests/leaf.rs` enforces a closed
  dependency list. That is filed as `docs/loop/needs-decision/mcp-tool-surface.md` with three options, and it is a
  dependency decision rather than a design one — this ADR fixes the *surface*, which does not change with the answer.
- **`audition` needs a running `App`.** [ADR 0006](0006-plugin-and-tool-server.md) already states the in-process hop is
  the first implementation; with no `App` the tool returns `unavailable` rather than pretending.
- **Six tools, not sixteen.** Every additional tool is a schema an agent must be told about and a name that can never
  change. The six cover the stated workflow — describe, pick, anchor, find music, find effects, ask why — plus the one
  that lets a person hear it.
