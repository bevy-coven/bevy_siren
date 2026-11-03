# Iteration 82 — research: per-asset provenance, and the two ways a licence record lies

The item is [#14](https://github.com/bevy-coven/bevy_siren/issues/14): *"Read and persist per-asset licence and source
metadata."* p0, unblocked by ADR 0005 (#6) and iteration 81's index work.

## Sources

| Source | The rule it reports |
|---|---|
| `#14`'s own body | *"Store licence and source per asset in the index, with an explicit unknown value rather than an empty string"* |
| `#14`'s technical notes | *"do not invent a licence when a library's README covers a whole folder — an inherited-but-unverified licence is worse than an explicit unknown, because it looks authoritative"* |
| `#14`'s fourth criterion | *"Unknown provenance is never silently presented as permissive"* |
| ADR 0005 (`docs/adr/0005-models-and-licences.md`) | from the licence audit: `CC0-1.0`, `MIT`, `Apache-2.0` permit commercial use; `CC-BY-NC-4.0` does not; an identifier with no tag grants nothing |
| ADR 0004 (`docs/adr/0004-vector-store.md`) | the store is one versioned file with a header and a checksum; a trailing optional section is the compatible shape |
| this crate, `semantic::index::MAGIC` | `"bevy_siren index 1"` — the version that a record-layout change would have to move |

## The technique, in the sources' terms

A licence record is three optional strings and a **three-valued** answer. The temptation is to make it a `bool`
`commercial_ok` defaulting to `true`, and that is the bug `#14` names: a default-true boolean turns *"nobody recorded
this"* into *"this is fine"*, which is the one direction that gets a studio sued.

So `permits_commercial_use()` returns `Option<bool>` and the three answers are:

| Answer | Meaning | Who decides |
|---|---|---|
| `Some(true)` | a licence identifier this crate recognises as commercial-friendly | the code, because the identifier is unambiguous |
| `Some(false)` | one it recognises as non-commercial | the code, for the same reason |
| `None` | nothing recorded, an empty record, or an identifier it does not know | **a person** |

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| the three fields | licence, source, author — all optional | `#14`'s scope |
| the unknown representation | **absent**, never an empty string standing in | `#14`'s first criterion |
| commercial licences recognised | `CC0`, `CC-BY*`, `MIT`, `Apache`, `BSD`, `ISC`, `Unlicense`, `MPL-2` as a **leading stem** | ADR 0005's audit |
| non-commercial marker | `-nc`, `NC-`, or the word `noncommercial` as a **substring** | ADR 0005 |
| sidecar shapes | CSV with a named header; JSON as object-or-array | what a purchased pack ships |
| sidecar names looked for | `licences.csv`, `licenses.csv`, `licences.json`, `licenses.json` | design default — the two spellings a pack may use |

## What good sounds like

Unknown stays unknown all the way to a person. An asset with no sidecar row reads `unknown, source not recorded` and
`permits_commercial_use() == None`; an asset whose sidecar wrote an empty licence is **recorded** as empty and still
answers `None`; and no identifier this crate does not recognise is ever reported as usable.

The failure this iteration found in its own first draft is the sharpest example of why the matching matters: the
permissive check was written as `lower.contains("mit")`, so **`"resubmitted-terms"` would have been accepted as the MIT
licence**. A substring match on a licence identifier is not a safe direction — it fails by *granting* permission — so the
check is a leading stem, and a test now pins `resubmitted-terms → None`.
