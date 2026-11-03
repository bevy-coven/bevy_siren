# review.md — iteration 66, three passes

## Pass 1 — Function: does the tokeniser produce the model's own input?

**The assertion that matters is equality, not plausibility.**

    crate ids == reference ids: True

Ten strings pinned against `transformers`' own `ClapProcessor`; four of them then fed to the real model, which ranked
the right clip first **4 / 4** (`evidence/tokenizer-e2e.txt`).

| Case | Test | Result |
|---|---|---|
| 50,265 entries, 50,000 merges | `the_vocabulary_is_the_models_own` | exact |
| ten strings | `ten_strings_match_the_reference_ids_exactly` | exact |
| a word that splits | `a_word_that_splits_is_not_looked_up_whole` | `tense` → 2 pieces |
| RoBERTa specials | `the_specials_are_the_roberta_pair` | 0 … 2 |
| punctuation keys | `a_vocabulary_with_a_colon_in_a_token_parses` | `:`=35, `,`=6, `.`=4 |

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Three defects, all found by testing rather than by reading:

| Defect | Found by | What it did |
|---|---|---|
| `splitn(2, ':')` on a vocabulary entry | the parse failing on the real file | took `"` as the key for `":"` and `","`, which are real tokens |
| `find("vocab")` | the parse failing again | found the `vocab` inside `added_tokens` instead of `model`'s |
| splitting merges on commas | an empty merge table | the array is pretty-printed across lines, so inner commas broke it |

And one that was **my test, not the code**: an early harness hard-coded `rain falling → [9946, 7349, 154]`, which is what
`falling` tokenises to *alone*. With the leading space it is the single token `3064`. The crate was right; the harness
was wrong, and it reported `ids == reference: False` until it was fed the crate's own output.

**The lesson, and it is the same one this loop keeps paying for:** the space-sensitivity means the *same word* is one
token or two depending on context. A tokeniser that split on whitespace and looked words up would be wrong on every
phrase and nothing downstream would fail.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **The seam is one call.** `Query::new(text)?.tokenize(&vocabulary)?` gives the ids `Model::embed_text` takes. The
  token budget is checked at the one place the count is known, excluding `<s>`/`</s>` because they are the model's.
- **A wrong vocabulary is refused**, not misused: a non-BPE file names the problem, and `a_file_that_is_not_a_byte_level_bpe_vocabulary_is_refused`
  proves it.
- **The default build is untouched**: 174 tests with no features, 413 dependency lines.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| issues | the tokeniser — the last seam on the semantic pipeline — built and pinned |
| new module | `src/semantic/tokenizer.rs` (+ `Query::tokenize`) |
| tests | **+11**, from 282 to **293** all-features |
| fixture | `tests/fixtures/clap-tokenizer.json`, 3,558,802 bytes, the model's own |
| evidence | `crate ids == reference ids: True`, retrieval **4 / 4** |

## A defect this iteration found in the harness, which is the next item

While looking at the render output, three different cues produced **byte-identical** WAVs
(`md5 eefe987d5aef348c1d6f4abe482db585` for `cue-footstep-grass-0`, `cue-dialogue-greeting-0` and `cue-bang-0`).

`examples/render.rs:99` synthesises every cue as `wave(440.0 * ratio, ms, 12.0)` — **a constant 440 Hz** — and
`write_gain` discards `speed` outright (`let _ = speed;`) although the module doc says "the gain and speed a play gives
it". So two cues with the same gain are the same file, and **every spectrogram this loop has judged was one sine
wave**, not the cue it claimed to show.

That is a bug in the evidence harness, not in the crate's playback — but it means the visual gate has been weaker than
it appeared. It is recorded here and is the first item of the next iteration.
