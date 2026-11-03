# Iteration 83 — research: the optional section, and why absence must survive a file

Completes [#14](https://github.com/bevy-coven/bevy_siren/issues/14)'s two blocked criteria by implementing the
recommendation in `docs/loop/needs-decision/provenance-in-the-index.md`.

## Sources

| Source | The rule it reports |
|---|---|
| `needs-decision/provenance-in-the-index.md` | recommends option 1A (a trailing `provenance` section, absence as absence) and 2A (`Match` gains the field, losing `Copy`) |
| `docs/loop/log.md`, three entries | *"actionable and non-breaking, so it need not wait"* — this repository's own stated precedent for a recommendation of this shape |
| ADR 0004 (`docs/adr/0004-vector-store.md`) | an optional trailing section is the compatible shape; the header's `MAGIC` does not move |
| `src/semantic/index.rs` | `MAGIC = "bevy_siren index 1"`; the body is checksummed by `fnv1a`; every non-header line begins with a 16-hex hash |
| `#21`'s measured finding (iteration 77) | a sentinel in a numeric column is how `−70 LUFS` became a lie: absence must be absence, not a value that looks like data |

## The technique, in the sources' terms

The store is line-oriented, so a new section is a new kind of line **after** the vectors, introduced by a marker the
existing parser can never mistake for a record: every vector line starts with 16 hex characters, and `provenance ` does
not. The section header carries its own count, so a truncated section is detected by the same rule that catches a
truncated vector body.

Three format decisions, each forced by a requirement:

1. **The section is inside the checksummed body.** A licence edited by hand is then a corrupt file, not a silent
   alteration of which assets may ship.
2. **An index with no provenance writes no section**, so it is **byte-identical** to what the crate wrote before the
   feature existed. That is what makes the change additive rather than a format migration.
3. **Fields are escaped**: space becomes `\s`, a backslash `\\`, and a recorded absence is `-`, with a literal `-`
   written `\-`. Without the escape, a pack called `kenney sounds` would split into two fields and a licence of `-`
   would read as *nothing recorded* — the same conflation the type refuses in memory, reintroduced by the file format.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| section marker | `provenance <count>` on its own line | this design |
| record layout | `<16 hex hash> <licence> <author> <source>` | this design |
| absent field | `-` | this design |
| recorded-unknown source | `?unknown` | the `Source::Unknown` variant |
| source kinds | `u<url>`, `p<pack>` | the `Source` variants |
| escape set | space, backslash, newline, lone `-`, empty | forced by the space-delimited layout |
| `Match` and `Copy` | `Copy` removed | `needs-decision/provenance-in-the-index.md`, option 2A |

## What good sounds like

An index written yesterday loads today and reports `provenance.is_empty()`; an index written today with records loads
and returns **exactly** what was stored, including a licence with a space in it and an explicitly-recorded empty
licence, which stays distinguishable from an absent one. A search result carries its licence **without being asked**, so
the failure mode is not "the consumer forgot" but "the type will not compile".
