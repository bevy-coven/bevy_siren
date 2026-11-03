# Iteration 82 — plan

## The ranked list

| # | Item | Why it ranks here |
|---|---|---|
| **1** | **#14 per-asset provenance** | The last unblocked p0. p0 *because* it is cheap now and impossible later: an index built without provenance can only be recovered by rescanning files that may be gone. |
| 2 | #21's index round trip | Waiting on `needs-decision/loudness-in-the-index.md`, and this iteration writes the sibling decision that can share its shape. |
| 3 | The binary index writer | ADR 0004's shape, still not code. |
| 4 | #22 spectral features | Unblocked, larger. |
| 5 | #28 the MCP tool host | Largest user-facing win, consumes an index that must first be correct. |

**Why (1) beats (2) in one sentence:** #21's remaining half is blocked on a decision while #14 is blocked on nothing, and
#14 is the p0 whose cost grows with every index built without it.

## What is built, and what is decided

**Built: `semantic::provenance`** — additive, no existing type touched.

| Item | Shape | Why |
|---|---|---|
| `Provenance` | `{ licence, source, author }`, all `Option` | `#14`: *"an explicit unknown value rather than an empty string"* |
| `Source` | `Url(String)` / `Pack(String)` / `Unknown` | `Unknown` is a **record that exists** and says the origin is not known — different from `None` |
| `permits_commercial_use()` | `Option<bool>` | three answers, and `None` means a person decides; a `bool` defaulting to true is the bug |
| `Sidecar::read` | CSV or JSON by extension | what a purchased pack ships |
| `ProvenanceProblem` | `Conflicting`, `Unreadable` | two rows for one file is a conflict, not last-write-wins |

**Decided elsewhere, per the crate's rule:** `docs/loop/needs-decision/provenance-in-the-index.md`.

1. **Where it is stored.** `#14` asks for it *in the index*; `Index::records` is `hash → vector` and the header is a
   pinned string, so adding a section is a **schema change**. Recommendation: an optional trailing `provenance` section,
   absence encoded as absence — the same shape ADR 0004 chose for loudness.
2. **How a result reports it.** `rank::Match` is `Copy { hash, score }`; attaching `Provenance` makes it neither. That is
   a **public-type change**. Recommendation: give `Match` the field and accept losing `Copy`, because the alternative —
   a caller looking it up separately — is the option that lets a consumer forget, which is exactly criterion four.

Both acceptance criteria that depend on those answers (stored, and exposed on a result) therefore wait for the decision.
The two that do not — explicit unknown, and never-permissive — are built and tested.

## Modules touched

| File | Change |
|---|---|
| `src/semantic/provenance.rs` | **new** — the type, the sidecar readers, 11 unit tests |
| `src/semantic/mod.rs` | `pub mod provenance;` |
| `docs/loop/needs-decision/provenance-in-the-index.md` | **new** — the two schema questions |

No existing type changes. No new dependency. No new threshold: the only constants are the licence stems and the two
sidecar spelling pairs, both of which are listed in `research.md` with their source.

## Carved off

- **Writing provenance into the index** and **exposing it on `Match`** — both blocked on the decision file.
- **A network lookup of a licence**, which `#14` puts explicitly out of scope.
- **Reading a folder-level `LICENSE` or `README`** — the pitfall `#14` names, and the reason `Sidecar` only accepts a
  per-file record.
