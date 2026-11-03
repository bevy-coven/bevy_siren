# Iteration 86 — review

## Pass 1 — Function: does the sidecar now reach the index?

**Critique.** The gap was exact and iteration 85 named it: `Sidecar::read` was tested since iteration 82 and **nothing
called it**. This iteration calls it, and the join is the only place two identities meet:

    Asset.paths (relative, several per content)  ×  Sidecar.records (the names a sidecar spells)
                          ↓
                    Asset.hash  →  Index.provenance

Each test asserts something a store-only or a name-blind implementation would fail:

| Claim | Assertion | Why it is not vacuous |
|---|---|---|
| the licence reaches the right asset | `provenance.len() == 1`, keyed to the named file's hash | a name-blind join gets 0 or 2 |
| an unnamed file is **absent**, not defaulted | `!provenance.contains_key(other)` and downstream `None` | a defaulted record would answer `Some(true)` and ship an unlicensed asset |
| a typo is counted | `unattributed == 1` and the summary says so | silently ignoring it makes a licence look recorded and attach to nothing |
| an unreadable sidecar ≠ no sidecar | `sidecar.is_some()` while the audio still indexed | conflating them loses every licence with no record |
| no sidecar is normal | `describe() == "1 indexed"` | the distinction the previous row depends on |

**Verdict: GREAT.**

## Pass 2 — Zoom: what the compiler told us

**Critique.** Adding two fields to `Built` broke two test literals with `missing fields 'sidecar' and 'unattributed' in
initializer of Built`. That is the right outcome and worth naming: `Built` exists to **report what a run did**, so a new
fact about a run should be impossible to add without a construction site noticing. A builder or `#[non_exhaustive]`
would have hidden it, and for this specific struct hidden is wrong.

**The pack-name test is the one that reads closely.** `"kenney sounds"` travels: CSV reader → `Source::Pack` → the
escaped `pkenney\ssounds` field → `parse_provenance_line` → back to `Some(Pack("kenney sounds"))`. Asserted at **both**
ends — the built index and the reloaded file — because the escaping bug that would split it lives only in the format.

**A limitation stated rather than glossed**: the path normalisation is a `\\` → `/` string replace. A sidecar spelling an
absolute path does not match, and that is an **unattributed record** — counted and reported — rather than a guessed
match. Guessing here would attach a licence to the wrong file, which is worse than attaching none.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: what does a library author do?

**Critique.** Drop a `licences.csv` beside the audio and build:

```csv
file,licence,source,author
step.wav,CC0-1.0,kenney sounds,Kenney
```

The named file's result reports `CC0-1.0, from \`kenney sounds\``; every other file reports `unknown, source not
recorded` and answers `None` to `permits_commercial_use()`. A typo in the sidecar appears in the summary as *"1 licence
records matched no file"*. A sidecar with no `file` column appears as *"Licence record unreadable: ... the header names
no \`file\`, \`path\` or \`filename\` column"*. A folder with no sidecar says nothing at all.

That is the whole authoring surface, and each failure mode is a sentence rather than a silence.

**The honest limit**: this iteration wires **provenance**. Loudness was wired in 85. What remains unwired is nothing —
every reader in `semantic::` is now called from somewhere production-shaped.

**Verdict: GREAT.**
