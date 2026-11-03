# review.md — iteration 68, three passes

## Pass 1 — Function: does one call answer a description?

| Claim | Test | Result |
|---|---|---|
| one call, one order | `a_description_reaches_an_answer_in_one_call` | 3 ranked, the indexed phrase first |
| empty is refused **before** the model | `an_empty_description_is_refused_before_the_model_runs` | `Query(Empty)` |
| no words is refused **before** the model | `a_description_with_no_words_is_refused_before_the_model_runs` | `Query(NoWords)` |
| a foreign index is refused **before** the embed | `an_index_from_another_model_is_refused_before_any_embedding` | `StampMismatch` |
| over-long is refused by count | `a_description_longer_than_the_model_accepts_is_refused_by_count` | `TooLong { need, 77 }` |
| the floor accounts for every vector | `the_answer_reports_what_the_floor_dropped` | `matches + dropped == 3` |
| determinism | `the_same_description_answers_the_same_way_twice` | field for field |

**And on the real model**, `evidence/ask-e2e.txt`: **4 / 4**, through the crate's own tokeniser.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the awkward cases

Three defects, all found by running rather than reading:

| Defect | Found by | Fix |
|---|---|---|
| the fixture's `MatMul` was **`[3, 512]`** — it accepted exactly three tokens | `a_description_reaches_an_answer_in_one_call`, the first time a real phrase (6 ids) reached it | rebuilt to average over the token axis, so it takes any length like the declared `[-1, -1]` |
| my floor test assumed the old fixture's **distinct** vectors | it failed once the fixture averaged | the assertion is now the floor's *accounting*, not a ranking the fixture cannot produce |
| `m.loaded().dimension;` — a **statement with no effect** | clippy | removed |

The first is the interesting one: the fixture had been **modelling the wrong contract since it was written**, and
nothing caught it because until this iteration nothing passed it a phrase longer than three tokens. A fixture that
disagrees with the graph it stands in for is worse than no fixture — it passes and proves the wrong thing.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it without me?

- **One call, one type.** `Embedder::new(&vocabulary, &mut model).ask(&index, &stamp, "…", 3, Floor::default())` returns
  an `Answer` or an `AskProblem` whose `Display` names the fix. The three sentences were read, not just the variants.
- **The ordering is documented as the design**, with the four wrong orders named, so the next person does not
  "simplify" it back.
- **The default build is untouched**: 181 tests with no features, and the seam is behind `semantic`.
- **The floor's aggressiveness is now a measurement**, not a hunch: at 0.126 it drops 2, 2, 1 and 3 of 3
  (`evidence/ask-e2e.txt`). That is the argument for `Floor::default()` staying 0.0, recorded where it can be checked.

**Verdict: GREAT.**

## The run

| | |
|---|---|
| item | the description → ids → model → search seam |
| new module | `src/semantic/ask.rs` (`Embedder`, `AskProblem`) |
| tests | **+9**, from 300 to **309** all-features |
| real-model proof | **4 / 4** on `bmb`, `evidence/ask-e2e.txt` |
| fixture | rebuilt to accept any token count, matching the graph's `[-1, -1]` |
