# plan.md — iteration 66: the tokeniser, and the next critical path

## The original path is closed; this is the next one

Iteration 65 closed all nine issues of `#2 → #3 → #7 → #9 → #10 → #11 → #12 → #15 → #17 → #26`. The semantic engine runs
end to end — scan, decode, convert, embed, index, rank, answer — except for **one seam**: something has to turn a
description into `input_ids`.

## The pick, and why it beats the runner-up

**Pick:** the tokeniser. It is the only thing between `Query::text()` and `Model::embed_text`, so every text query
needs it and nothing downstream — the tool surface, the panel, the labelled set — can be built until it exists.

**Runner-up:** `#41` (tunable weights). It loses on dependency order: weights refine a ranking, and a ranking needs a
query to rank.

## What it is, and why it is not a dependency

CLAP's tokeniser is a `RobertaTokenizer`: **byte-level BPE**, 50,265 vocabulary entries, 50,000 merges, RoBERTa's
`<s>`/`</s>` specials. `transformers`' `tokenizer.json` (**3,558,802 bytes**) carries the whole rule set, so the
implementation is the algorithm plus a data file — not a crate. The crate's dependency list stays at five.

## The tests that prove it

| Behaviour | Test |
|---|---|
| the vocabulary is the model's own | `the_vocabulary_is_the_models_own` — 50,265 entries, 50,000 merges |
| **ten strings match the reference ids exactly** | `ten_strings_match_the_reference_ids_exactly` |
| a word that splits is not looked up whole | `a_word_that_splits_is_not_looked_up_whole` |
| the specials are RoBERTa's pair | `the_specials_are_the_roberta_pair` — 0 … 2 |
| the mask attends every token | `the_attention_mask_attends_every_token_there_is` |
| empty text is not an error | `an_empty_description_encodes_to_no_ids_and_the_specials_alone` |
| a non-BPE file is refused | `a_file_that_is_not_a_byte_level_bpe_vocabulary_is_refused` |
| punctuation keys parse | `a_vocabulary_with_a_colon_in_a_token_parses` — `:`, `,`, `.` |
| a description becomes the model's ids | `a_description_becomes_the_ids_the_model_takes` |
| an over-long description is refused by count | `a_description_longer_than_the_model_accepts_is_refused_by_count` |
| the specials do not eat the token budget | `the_token_count_excludes_the_specials` |

## Every number, and its source

| Number | Value | Source |
|---|---|---|
| vocabulary | 50,265 | measured from the model, `research.md` S1 |
| merges | 50,000 | measured, S1 |
| `<s>` / `</s>` | 0 / 2 | measured, S1 |
| `,` / `:` / `.` | 6 / 35 / 4 | measured, verified against the model |
| `falling` vs ` falling` | `[7349,154]` vs `[3064]` | measured, S2 |
| token limit | 77 | the model card, already `MAX_TEXT_TOKENS` |

## What was carved off

**Loading the fixture at runtime.** The 3.5 MB `tokenizer.json` is checked in as a test fixture, not embedded in the
binary. A game supplies its own copy of the model's file, which is right: the vocabulary is the model's data, not the
crate's, and pinning it in the crate would make a model upgrade a crate release.

## A disk note that cost a build

The gate failed with **`No space left on device`** on `proc-macro2`, which reads as a linker error in a dependency and
is not one — the disk was at **121 MB**. `cargo clean` (20.8 GiB) fixed it. This is the third time this project has
hit it, and the skill's own note says why.
