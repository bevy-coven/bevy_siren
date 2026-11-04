# Decision: what the default similarity floor should be

## The question

`Floor::default()` — and `Weights::min_similarity`, which `Toolset::default()`, `siren-tools` and every `Ranked` request inherit — is
**`0.0`**, which keeps everything. It was documented as *"a measurement, not timidity"*: six synthetic clips whose right and wrong
score bands overlapped, from which *"no single threshold separates them"* and *"a floor of 0.5 would reject every correct match"*
were concluded. **2 000 real recordings said otherwise** (`docs/guide/golden-set.md`, iteration 111), and **303 game sounds, measured in
iteration 113, say otherwise again — in the other direction.** Should the default become a positive number, and which?

This is a **public default**: every consumer that does not set a floor inherits it, and a shorter result list is a behaviour change a
game can see. So the loop files it rather than changing it.

## What was measured

ESC-50 (2 000 clips, 50 classes, human-labelled), the real CLAP model, and **two sets of descriptions of the same 50 classes**: the
maintainer's own (`golden/esc50/queries.tsv`) and a paraphrase that avoids the class words, the way a stranger would write them
(`queries-paraphrase.tsv`) — each with the same 16 descriptions of sounds the library does not contain. Same index, same model; only the
questions differ. Run with `examples/goldenbench`; the independent analysis is `evidence/after/floor_analysis.py`.

| floor | results per query (of 2 000) | precision | no-answer queries silenced | descriptions that still get a result | hit@3 kept | top-1 kept |
|---|---|---|---|---|---|---|
| | maintainer / stranger | maintainer / stranger | (same 16 both times) | maintainer / stranger | maintainer / stranger | maintainer / stranger |
| 0.00 (today) | 1 440 / 1 318 | 2.8 % / 3.0 % | 0 of 16 | 50 / 50 | 100 % / 84 % | 90 % / 72 % |
| 0.30 | 107 / 93 | 34.6 % / 33.7 % | 0 of 16 | 50 / 50 | 100 % / 84 % | 90 % / 72 % |
| **0.35** | 66 / 55 | 51.8 % / 48.5 % | **4 of 16** | 50 / 50 | 100 % / 84 % | 90 % / 72 % |
| 0.40 | 43 / 33 | 68.6 % / 62.0 % | 7 of 16 | 50 / 50 | 100 % / 84 % | 90 % / 72 % |
| 0.45 | 28 / 18 | 81.6 % / 74.4 % | 9 of 16 | 50 / **48** | 100 % / **80 %** | 90 % / **70 %** |
| 0.50 | 17 / 10 | 90.4 % / 85.4 % | 11 of 16 | 49 / **41** | 98 % / **70 %** | 90 % / **64 %** |

- **The weakest top result** of any description scores **0.492** in the maintainer's wording and **0.405** in the stranger's. The margin
  a floor has under it is therefore 0.105 at 0.30, **0.055 at 0.35**, and **0.005 at 0.40**.
- A floor on a query's top score separates a description the library can answer from one it cannot with an **AUC of 0.963** (the
  stranger's wording: 0.885). A margin (top minus tenth) does not: 0.496 / 0.461.
- **Wording matters more than the floor does.** hit@1 / hit@3 / hit@10 are 90 / 100 / 100 % in the maintainer's words and **72 / 84 / 92 %**
  in the stranger's. The right answers' median similarity falls 0.468 → 0.402 and their 5th percentile 0.273 → 0.158.
- The no-answer queries find their nearest neighbour, not nothing — a gunshot finds `fireworks` at 0.593, a lion `cow` at 0.530 — so no
  absolute floor silences them all (0.55 leaves 1 of 16, and costs 5 to 20 of 50 answerable descriptions).

**What the measurement does not cover:** one library of environmental recordings, **partly inside the model's training data** (the
paper excluded the overlap from its own figure), no music, no speech, no game foley, and two wordings by the same person. A library whose
correct answers score lower — stylised or synthesised sounds — would lose answers to a floor chosen on this one; the second wording is a
small demonstration of how fast that happens.

**And this repository already holds a counter-example, from iteration 110, that iteration 111 did not set against the numbers above.**
On **fourteen macOS system sounds** five descriptions were run against the real model (`docs/loop/runs/2026-11-03-110/evidence/after/parity.txt`; the
crate's scores agree with the `transformers` reference to 4.9 × 10⁻⁴, so they are the model's and not a bug), and **no result of any of
them scored above 0.256**:

| description | best result | score |
|---|---|---|
| "a deep sonar ping underwater" | `Morse.wav` (then `Hero.wav` 0.252) | 0.256 |
| "a short electronic beep" | `Sosumi.wav` — the answer the guide calls a match | **0.206** |
| "a frog croaking" | `Purr.wav` (`Frog.wav` itself ranks eleventh, at −0.02) | 0.145 |
| "a bottle being blown across" | `Purr.wav` (`Blow.wav` third) | 0.081 (`Blow` 0.021) |
| "a glass being tapped" | `Glass.wav`, by name | **0.016** |

**Every positive floor in the table above — 0.30 and up — would have returned nothing for all five.** One small library of synthetic
alert sounds, five casual descriptions, and a judgement of which result is right that is mine: a caveat, not a measurement. But it is the
only second library this crate has run, it is the kind a game ships (synthetic UI sounds), and it points the opposite way from the table.

## The second library, measured (iteration 113)

**303 game sounds from seven of Kenney's CC0 packs**: 29 classes, and 47 descriptions in each of two wordings (29 with answers, 18 without), the same
model and the same `goldenbench`. The queries were committed before the model ran, with a prediction (`docs/loop/runs/2026-11-03-113/research.md`).

| | ESC-50 (maintainer's / stranger's) | Kenney (maintainer's / stranger's) |
|---|---|---|
| the right answers' median cosine | 0.468 / 0.402 | **0.191 / 0.193** |
| the weakest top result of an answerable description | 0.492 / 0.405 | **0.261 / 0.238** |
| the strongest top result of a description that should find nothing | 0.593 | **0.490** |
| answerable descriptions whose best result is under 0.35 (Wilson 95 %) | 0 of 50 (0 – 7 %) | **13 / 6 of 29 (28 – 62 % / 10 – 38 %)** |
| AUC of the top score (95 % interval over the descriptions, 4 000 resamples) | 0.963 / 0.885 (0.90 – 1.00 / 0.78 – 0.96) | 0.759 / 0.808 (0.59 – 0.90 / 0.65 – 0.94) |
| hit@1 · hit@3 | 90 · 100 / 72 · 84 % | **31 · 52 / 31 · 38 %** |

What each floor did on the second library (maintainer's / stranger's):

| floor | results per query (of 303) | precision | no-answer queries silenced | hit@3 kept | top-1 kept |
|---|---|---|---|---|---|
| 0.00 (today) | 133 / 140 | 6.9 / 6.6 % | 0 of 18 | 51.7 / 37.9 % | 31.0 / 31.0 % |
| 0.20 | 23 / 24 | 21.9 / 20.8 % | 0 | 51.7 / 37.9 % | 31.0 / 31.0 % |
| 0.25 | 14 / 15 | 28.2 / 25.2 % | 4 | 51.7 / 34.5 % | 31.0 / 27.6 % |
| 0.30 | 7 / 9 | 38.7 / 26.2 % | 9 | **44.8** / 34.5 % | 31.0 / 27.6 % |
| **0.35** | 3 / 4 | 52.7 / 26.0 % | 15 | **27.6 / 31.0 %** | **20.7 / 24.1 %** |
| 0.40 | 1 / 2 | 50.0 / 29.2 % | 15 | **24.1 / 13.8 %** | **17.2 / 13.8 %** |

Read against ESC-50's table above: **the floors that were free there are not free here, and none silences a no-answer query without deleting an answer.**

- **No floor exists on either library** (the weakest answerable top is below the strongest unanswerable one). On the second the AUC is 0.759 / 0.808
  because **one class — `engine` — is the first result for 22 of the 47 descriptions**, a hub: thirty distinct files supply all 47 first results and 238
  of the 303 files are in no description's top three. *A helicopter* finds an `engine` at 0.490, above the best result of 27 of the 29 answerable
  descriptions; *a dog barking* finds one at 0.216.
- **The largest floor that costs neither library an answer in either wording is 0.20** — and at 0.20 it silences **no** no-answer query on either. *The count behind it is 0 of 29 descriptions under 0.20
  on Kenney, which a Wilson interval says is compatible with up to 12 % of a library's descriptions: the largest floor that cost nothing here, not a safe one.*
- **A floor relative to the query is worse on Kenney**: centring each query on its own median (`eval::centred`) lowers the AUC from 0.759 to 0.626 (0.808 to 0.703) — outside the resampling noise in both
  wordings (−0.13, interval −0.25 – −0.03; −0.11, −0.22 – −0.01) — where on ESC-50 it made no clear difference (0.963 → 0.968; 0.885 → 0.924, an interval of −0.001 – +0.09). **The AUC alone does not separate the
  libraries** (the intervals overlap in the stranger's wording); the share of descriptions under 0.35 does, in both.
- **A normalisation on the gallery side helped both libraries, in an exploratory test** (`evidence/after/normalisations.py`, Python on the crate's own
  similarities): subtract from each file's score its mean similarity to the *other* descriptions of the set — Querybank Normalisation's idea (Bogolin et al.,
  CVPR 2022, DOI `10.1109/cvpr52688.2022.00513`) in its simplest form. AUC: Kenney 0.759 → **0.925** and 0.808 → **0.910**, ESC-50 0.963 → 0.969 and 0.885 →
  0.939; hit@1: Kenney 31 → 44.8 % (maintainer's), ESC-50 72 → 80 % (stranger's). **Caveats that matter**: the bank is the set's own other descriptions, so
  it knows the kind of question that will be asked; there are 29 and 50 queries; and **no floor exists even then** (Kenney: the weakest answerable top is
  +0.183, the strongest unanswerable +0.342). Not built.
- **These are the model's numbers, not the crate's**: Python recomputes every figure the report prints and agrees, and against `transformers` the 48 kHz
  files are identical and every retrieval metric agrees within a query.

## Options

1. **Keep `0.0`.** Cost: a description the library cannot answer returns `k` results that look like answers (*a lion roaring* → `cow` 0.53; *a dog barking* →
   a spaceship `engine` 0.22); an agent or a panel cannot tell, and the list a caller asks for with a large `k` is 93 – 97 % wrong. Benefit: no behaviour change
   and **no risk from library shift** — the one property the second library showed matters; a project that wants a floor sets one (`Weights::min_similarity`,
   `Ranked.floor`).
2. **`0.20`.** The largest floor that costs neither library an answer in either wording (ESC-50 keeps everything to 0.40; Kenney to 0.25 in the maintainer's
   wording and to 0.20 in the stranger's). Cost: a **0.038 margin** under the weakest top result anywhere (Kenney, stranger's, 0.238) — a third, less friendly
   library loses answers — and it silences **none** of the 34 no-answer queries, so it only shortens a list (1 440 → 330 results a query on ESC-50, 140 → 24 on
   Kenney). Benefit: it hides the negative-cosine tail, and no answer is lost on two libraries in two wordings.
3. **`0.30` – `0.45`.** *Rejected by the second library.* `0.35`, iteration 111's recommendation, is free on ESC-50 and returns nothing at all for 13 of the 29
   descriptions on Kenney (hit@3 51.7 → 27.6 %); `0.30` already loses 7 points of hit@3 there; `0.40` has a 0.005 margin on ESC-50's own stranger's wording and loses
   about half of Kenney's top-1 hits (31.0 → 17.2 % and 13.8 %). Nothing that silences half of the no-answer queries keeps the answers of a game library.
4. **A floor relative to the query** (each score less that query's median, `eval::centred`). *Measured and rejected*: AUC 0.626 against 0.759 on Kenney.
5. **A per-library floor from a bank of no-answer descriptions** (a percentile of their top scores). *Rejected by the same data*: the strongest unanswerable top on
   Kenney, 0.490, is above the top result of 27 of 29 answerable descriptions, so a floor that silences them deletes nearly every answer.
6. **A gallery-side normalisation — a hubness correction**: at index time store, per file, its mean similarity to a bank of generic descriptions, and rank and
   floor on the score less that mean. Cost: an index-format change and a bank (public API, and the stamp rules for an index apply), and an evaluation on a bank that is
   *not* the questions asked. Benefit: in the exploratory test the only change that helped both libraries in both wordings, most where the hub is worst (Kenney's AUC
   0.76 → 0.93). **Not built.**

## Recommendation

**Keep `0.0` (option 1). The second library turns iteration 112's "wait" into a decision the data makes, and withdraws iteration 111's `0.35` for good.** No positive
absolute floor survives two libraries in two wordings except one that changes nothing a person sees; what would let a floor do its job — telling a description the library
can answer from one it cannot — is not a number on the cosine at all.

- **`0.20` (option 2) is the only positive default the evidence supports, and it is cosmetic**: it silences nothing. If the owner wants a shorter list by default that is the
  value, and its margin (0.038) is the thinnest in this file.
- **The next measured step is option 6**, with the hub as its target: an index-time statistic and a bank **held out from the evaluation**, tested on both libraries in both
  wordings, with the success criterion stated in advance — *some* threshold on the normalised score that keeps every answerable description's top result and silences a real
  share of the unanswerable ones. If that holds the floor becomes a number worth defaulting; if not, the floor stays a per-project setting and the guide says so.
- **What would overturn this:** a third library of game audio whose right answers score near ESC-50's (the second library's answer would then be about Kenney's packs and not
  about game audio), or a held-out-bank normalisation that does not survive the held-out bank.

## What is blocked

Nothing. The default stays `0.0`; `Floor::min_score`, `Weights::min_similarity` and `golden-set.md` state the measurement and point
here. A change is two lines (`Floor::default`, `Weights::default_min_similarity`), an update of the test that pins `0.0` in a defaults
test (`tests/semantic_weights.rs`), and a row in `docs/guide/semantic-config.md`. The loop did not change them.
