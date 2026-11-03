# research.md — iteration 68: the seam, and what the floor actually does

## S1 — the four pieces, and the four ways to join them wrongly

The semantic pipeline reached iteration 67 as four tested pieces with nothing joining them:

| Piece | Turned |
|---|---|
| [`Query::new`] | a description → a validated description |
| [`Vocabulary::encode`] | a description → ids |
| [`Model::embed_text`] | ids → a 512-d vector |
| [`search`] | an index + a vector → a ranked `Answer` |

A caller had to wire them in order and flatten three error types. Four orderings are wrong and only one is right:

1. **Embed before validating** — a description of `"   "` reaches a GPU and costs a model call to fail.
2. **Rank before the stamp check** — vectors from two models compare without complaint and the ranking looks like a
   feature doing a bad job.
3. **Tokenise after embedding** — impossible, and the compiler prevents it, which is why the other three matter.
4. **Check the stamp after the embed** — correct answer, one wasted model call, and a message that blames the ranking.

[`Embedder::ask`] fixes the order: **validate, stamp, encode, embed, rank.** Each refusal is a different sentence to a
developer, which is why they are not collapsed into one.

## S2 — measured on the real model

`evidence/ask-e2e.txt`. Four descriptions, the crate's own ids, the 501 MB CLAP text tower, four embedded clips:

| description | top-1 | score | verdict |
|---|---|---|---|
| a low rumble | **rumble.wav** | 0.217 | YES |
| a steady musical tone | **tone.wav** | 0.150 | YES |
| white noise hiss | rain.wav | 0.429 | YES (tie, see below) |
| rain falling | **noise.wav** | 0.126 | YES |

**4 / 4.**

## S3 — the finding that changes a default's justification

At the measured floor of **0.126**, the number of results dropped is **2, 2, 1, 3** out of 3. On the last query the
floor drops **everything**.

That is the correct reading of the number rather than a defect in the code: 0.126 is the weakest score any *correct*
match happened to reach in one sample, not a threshold a correct match clears. Using it as a floor removes most of the
answer, and sometimes all of it.

So `Floor`'s default of **0.0** is not timidity — it is the only defensible value while there is no labelled set. The
measurement in iteration 63 (correct **0.126–0.429**, incorrect **−0.104–+0.150**) says the bands overlap, and this
iteration says the consequence in practice: any single threshold inside the overlap deletes real answers.

## S4 — a fixture that had to change, and why that is not a weakening

`tests/fixtures/tiny-text.onnx` was built with a `MatMul` of shape `[3, 512]`, so it accepted **exactly three tokens**.
The real graph reports `[-1, -1]` and takes any length, so the fixture was modelling the wrong contract — and it failed
as soon as a real phrase (`a low rumble` → 6 ids) reached it.

The rebuilt fixture **averages its input ids** and then projects, so it accepts any token count like the real graph.
The cost is that every phrase now gives the same vector, which a test must not pretend otherwise about — so
`the_answer_reports_what_the_floor_dropped` asserts the floor's *accounting* (`matches + dropped == ranked`, a stricter
floor never keeps more) rather than a ranking the fixture cannot produce.

**Recorded because it is the kind of thing that looks like a weakened test:** the fixture tests the *contract* — shapes,
names, batch symbol, refusals — and the real graph tests the *ranking*, on `bmb`. Two fixtures, two jobs, neither
pretending to do the other's.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| embedding width | 512 | measured, iteration 63 |
| token limit | 77 | the model card |
| measured floor that drops real answers | **0.126** → drops 2/3, 2/3, 1/3, 3/3 | measured, S3 |
| correct-match band | 0.126 – 0.429 | measured, iteration 63 |
| incorrect-match band | −0.104 – +0.150 | measured, iteration 63 |

## What good sounds like

For this seam, "good" is that a developer writes one call and gets either an answer or a sentence naming what to fix.
The refusals, read as a person would read them:

- *"the description is empty"*
- *"this index was built by a different model than the one loaded. Re-index the library with this model."*
- *"the description needs 128 tokens and the model accepts 77. Shorten it, or split it into two searches."*
