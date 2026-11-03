# Iteration 86 — research: the join between two identities

The item is the last unwired reader: `Sidecar::read` existed and was tested since iteration 82, and **nothing called
it**. This iteration calls it from `build`.

## Sources

| Source | The rule it reports |
|---|---|
| `src/semantic/provenance.rs` | `Sidecar.records: BTreeMap<String, Provenance>` — keyed by **the file name the sidecar spells** |
| `src/library.rs` | `Asset.paths: Vec<PathBuf>` — paths **relative to the scan root**, and a content can have several |
| `src/library.rs::fnv1a` | the index is keyed by **content hash**, not path |
| `docs/loop/needs-decision/provenance-in-the-index.md` | storage is already decided and built; this is the production step |
| `#14`'s technical notes | *"do not invent a licence when a library's README covers a whole folder"* — the rule the reader already enforces |

## The technique, in the sources' terms

Two identities meet here and nowhere else: **a sidecar names files, the index names bytes**. The scan already holds the
join key — each `Asset` carries both its content hash and every path that content appears at — so the mapping is
`Asset.paths × Sidecar.records → Asset.hash`.

Three consequences, each of which becomes a reported fact rather than a silent one:

1. **A record that matches no file is counted.** The commonest cause is a typo in the sidecar; ignoring it makes a
   licence look recorded from the author's point of view and absent from every asset's.
2. **An unreadable sidecar is not "no licences".** One means every licence is missing; the other means none was expected.
   `Built::sidecar` carries the reader's own words.
3. **No sidecar at all is normal** and says nothing, so a healthy run reads exactly as it did before.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| sidecar names looked for | `licences.csv`, `licenses.csv`, `licences.json`, `licenses.json` | `SIDECAR_NAMES` |
| lookup key | a path relative to the scan root, with `\\` normalised to `/` | `Asset.paths` |
| an asset at several paths | matched if **any** path is named | `Asset.paths` is a `Vec` |
| unmatched record | counted in `Built::unattributed` | this design |
| unreadable sidecar | carried in `Built::sidecar`, build continues | this design |

## What good sounds like

A folder with `step.wav` and `licences.csv` naming `step.wav` gets **one** provenance record, attached to that file's
content hash, with the licence the sidecar wrote — and the folder's other files carry **nothing**, which downstream
reads as `unknown` and never as permission. A typo in the sidecar is **counted and named in the summary**. A sidecar
that cannot be read leaves the audio indexed and says so in the same sentence. A folder with no sidecar at all reports
`"1 indexed"` and nothing more.
