# plan.md — iteration 68: `Embedder::ask`, the seam in one call

## The pick, and why it beats the runner-up

**Pick:** the description → ids → model → search seam. It is item 4 of iteration 67's remaining list, and it is the
highest-value item because **every downstream issue depends on it**: `#29` `search_music`, `#32` the panel and `#45` the
regression harness all need one call that answers a description, and none of them should be re-implementing the order.

**Runner-up:** `#41` (tunable weights). It refines a ranking that nothing can currently ask for in one call. A weights
type with no consumer is a type, not a feature.

## The change

`src/semantic/ask.rs`:

| Type | What it is |
|---|---|
| `Embedder<'a>` | a [`Vocabulary`] and a [`Model`] **together**, because a description needs both and they must agree |
| `Embedder::encode` | description → ids, checked against the token limit |
| `Embedder::embed` | description → a 512-d vector |
| `Embedder::ask` | description + index → a ranked [`Answer`], or one named refusal |
| `AskProblem` | one error type for the path, each variant a distinct thing to do |

**The order is the design**: validate → stamp → encode → embed → rank. Four other orders are wrong, and only the
first is *cheap* to get right — a description with no words in it must not reach a GPU.

## Every number, and its source

No new constants. The seam adds no rate, curve or threshold: it composes functions whose numbers are already documented
(`MAX_TEXT_TOKENS` = 77 from the model card, `EMBEDDING_DIM` = 512 measured, `Floor` = 0.0 as a **design default**).

## The tests that prove it

| Behaviour | Test |
|---|---|
| a description reaches an answer in one call | `a_description_reaches_an_answer_in_one_call` |
| an empty description is refused **before** the model | `an_empty_description_is_refused_before_the_model_runs` |
| a description with no words is refused **before** the model | `a_description_with_no_words_is_refused_before_the_model_runs` |
| a foreign index is refused **before** any embedding | `an_index_from_another_model_is_refused_before_any_embedding` |
| an over-long description is refused by count | `a_description_longer_than_the_model_accepts_is_refused_by_count` |
| the floor's accounting is complete | `the_answer_reports_what_the_floor_dropped` |
| the same description answers the same way twice | `the_same_description_answers_the_same_way_twice` |
| `k = 0` is still a refusal at the top of the path | `a_zero_k_is_still_a_refusal_at_the_top_of_the_path` |
| an embedding is the width the index holds | `an_embedding_is_the_width_the_index_holds` |

## What was carved off, and what it cost

**The fixture, rebuilt.** `tests/fixtures/tiny-text.onnx` accepted exactly three tokens because its `MatMul` was
`[3, 512]`; the real graph reports `[-1, -1]`. Rebuilt to average over the token axis so it takes any length. The cost
is recorded rather than hidden: every phrase now gives the same vector, so the ranking is proved on `bmb` against the
real model (`evidence/ask-e2e.txt`, **4 / 4**) and the fixture proves the *contract*.

**Nothing else.** `#41`, `#40` and `#28` remain; the four `needs-decision/` files that wait on one public-enum break
still wait.
