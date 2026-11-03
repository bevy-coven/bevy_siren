# Iteration 82 — review

## Pass 1 — Function: which of #14's criteria hold, and which do not?

**Critique.** #14 has four criteria, and **two are answered here while two are genuinely blocked** — the split is stated
rather than glossed:

| # | Criterion | State |
|---|---|---|
| 1 | no metadata carries an explicit unknown, distinguishable from a recorded empty value | **met** — `an_absent_licence_is_distinguishable_from_an_empty_one` |
| 2 | a sidecar's fields reach the index without a hand-written mapping per library | **half** — the sidecar *reads* (CSV and JSON, by column name not position); reaching the *index* is a schema change |
| 3 | a search result can report an asset's licence and source | **blocked** — `rank::Match` is `Copy`; the change is in the decision file |
| 4 | unknown provenance is never silently presented as permissive | **met** — `unknown_provenance_never_reads_as_permissive` |

`#14` asks for storage *in the index*, and `Index::records` is `hash → vector` under a pinned `MAGIC` string. Adding a
field there is a schema change, and this crate's rule sends that to a decision file. **Deciding itinline would have been
the easier pass and the wrong one.**

**Critique of the shape.** `permits_commercial_use() -> Option<bool>` is the whole design. A `bool` defaulting to
`true` turns *"nobody recorded this"* into *"this is fine"*, which is the direction that gets a studio sued; a `bool`
defaulting to `false` would call `CC0` unusable. Three answers, with `None` meaning *a person decides*, is the only
honest one.

**Verdict: GREAT**, with two criteria explicitly outstanding and tracked.

## Pass 2 — Zoom: the over-claim it nearly shipped

**Critique.** The permissive check was `lower.contains("mit")` — a **substring** match. `"resubmitted-terms"` contains
`mit` and would have been accepted as the MIT licence. **Every test passed as written**, because none of them used a
string that happened to contain the letters.

That is the exact failure mode this function exists to prevent: it fails by *granting* permission, in code whose only job
is to refuse to over-claim. Fixed to a **leading stem** match, and the fix is pinned:

- `resubmitted-terms` → `None`
- `CC-BY-4.0` → `Some(true)` (previously mis-asserted as `None` in my own test)
- `CC-BY-SA-4.0`, `CC-BY-ND-4.0` → `Some(true)` — share-alike and no-derivatives are separate obligations, not
  commercial restrictions, and conflating them would have refused usable licences
- `CC-BY-NC-4.0`, `CC-BY-NC 4.0`, `NonCommercial` → `Some(false)`, checked **first** because a missed non-commercial is
  the dangerous direction

The asymmetry is deliberate and worth naming: the **non-commercial** check is a substring (a missed `-nc` is dangerous),
the **permissive** check is a stem (a spurious match is dangerous).

**Verdict: GREAT.** A real over-claim was found by re-reading the matching rather than by a test, and is now a test.

## Pass 3 — Fresh eyes: can a person use this without the code explained?

**Critique.** A sidecar author writes either of these, and both read:

```csv
file,licence,source,author
step.wav,CC0-1.0,kenney-sounds,
```

```json
[{"file": "step.wav", "licence": "CC0-1.0", "source": "https://kenney.nl"}]
```

Columns are found by **name**, so a pack that orders them differently still reads, and both `licence` and `license` are
accepted. An empty cell records nothing — the `Option` stays `None` — which is criterion one measured end to end.

**The pitfall #14 names is refused in code, not just in prose**: `Sidecar` reads only a per-file record. A folder-level
`LICENSE` or `README` is never consulted, so a pack whose README says "all CC0" gets **unknown provenance for every
file** rather than an inherited licence that looks authoritative. That is the behaviour the issue asks for, and a reader
can see *why* in the module doc.

**The honest limit, stated in the module**: the JSON reader is a brace-scanner for flat records, not a general JSON
parser, and the crate's dependency list is closed. A nested sidecar would be misread — and that is written down rather
than presented as a complete parser.

**Verdict: GREAT.**
