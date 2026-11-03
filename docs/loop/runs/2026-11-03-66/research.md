# research.md — iteration 66: the tokeniser, and the two bugs that made it exact

The original critical path closed in iteration 65. This iteration opens and advances the **next** one: the seam between
the crate and its consumers. The first item is the tokeniser, because every text query needs it and nothing downstream
can be built without it.

## S1 — CLAP's tokeniser, measured from the model

| | |
|---|---|
| class | `RobertaTokenizer` |
| model | **byte-level BPE** |
| vocabulary | **50,265** entries |
| merges | **50,000** |
| pre-tokeniser | `ByteLevel`, `add_prefix_space: false`, `use_regex: true` |
| post-processor | `RobertaProcessing` — `<s>` (0) … `</s>` (2) |
| `tokenizer.json` | **3,558,802 bytes** |

`transformers`' `tokenizer.json` carries the whole rule set — vocabulary, merges, the pre-tokeniser's pattern, the
specials — so a Rust implementation needs the algorithm and a data file, not a dependency.

## S2 — the bug that made the naive approach wrong

**A word is one token or two depending on whether a space precedes it.**

| text | ids | pieces |
|---|---|---|
| `falling` | `[7349, 154]` | `fall` + `ing` |
| ` falling` | `[3064]` | `Ġfalling` |

The merge table has `(fall, ing)` at rank 7207 and **no** `(Ġ, fall)` — `Ġfall` is a vocabulary entry (id 1136) reached
by `(Ġ,f)`, `(Ġf,a)`, … So the greedy BPE run over the *byte-level characters of the whole pre-tokenised piece* is what
produces `Ġfalling` in one step, and running it per whitespace-separated word never can.

A tokeniser that split on whitespace and looked each word up would produce `[7349, 154]` for `rain falling` and a wrong
embedding for every phrase — **and nothing downstream would fail**, because the ids are still in range and the model
still returns a unit vector.

## S3 — the bug in the parser, twice over

`tokenizer.json` is not regular enough for `split(':')`:

1. **`":"` and `","` are real tokens**, written `"\":\": 6`. Splitting each entry on the first colon takes `"` as the
   key and fails on the file. Reading the quoted key first, then the colon after it, is the fix.
2. **`"vocab"` appears twice** — inside `model`, and inside `added_tokens`. A plain `find` picks the wrong one, and the
   parse then fails on a key that is not a string. Scanning for the key at a known brace depth, outside strings, is the
   fix.
3. **The merges array is pretty-printed across lines** (`[\n "Ġ",\n "t"\n ]`), so splitting on commas breaks inside a
   pair. Walking the bracket pairs is the fix.

## S4 — proven, not assumed

| | |
|---|---|
| crate ids == reference ids, 10 strings | **True** |
| crate ids == reference ids, fed to the model | **True** |
| retrieval with the crate's own ids | **4 / 4** |

`crate ids == reference ids: True` is the assertion that matters: it means a description tokenised in Rust is the
*same input* the Python path would have produced, not merely a plausible one.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| vocabulary size | 50,265 | measured, S1 |
| merge count | 50,000 | measured, S1 |
| `<s>` / `</s>` ids | 0 / 2 | measured, S1 |
| `,` / `:` / `.` ids | 6 / 35 / 4 | measured, verified |
| `falling` vs ` falling` | `[7349,154]` vs `[3064]` | measured, S2 |
| `tokenizer.json` size | 3,558,802 bytes | measured, S1 |

## What good sounds like

For a tokeniser, "good" is **not** plausibility — it is **equality with the reference on inputs chosen to break naive
implementations**. The ten pinned strings cover: a word that splits (`tense`), a word that does not (`rain`), the
space-sensitivity above (`falling`), punctuation (`:`, `,`), a multi-word phrase, and a comma-separated list. The
retrieval check then confirms the ids are not merely equal but *useful*.

The literature search (`hs distill search`) returned nothing specific to audio-model tokenisation — the nearest hits
were on image sampling and interpolation. The authority here is the model's own data file, and it is vendored as a test
fixture so the assertion cannot drift.
