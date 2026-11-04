# Decision: what the default similarity floor should be

## The question

`Floor::default()` — and `Weights::min_similarity`, which `Toolset::default()`, `siren-tools` and every `Ranked` request inherit — is
**`0.0`**, which keeps everything. It was documented as *"a measurement, not timidity"*: six synthetic clips whose right and wrong
score bands overlapped, from which *"no single threshold separates them"* and *"a floor of 0.5 would reject every correct match"*
were concluded. **2 000 real recordings say otherwise** (`docs/guide/golden-set.md`, iteration 111). Should the default become a
positive number, and which?

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

## Options

1. **Keep `0.0`.** Cost: a description the library cannot answer returns `k` results that look like answers (`a lion roaring` → `cow`,
   similarity 0.53); an agent or a panel cannot tell, and the list a caller asks for with a large `k` is 97 % wrong. Benefit: no behaviour
   change, no risk from library shift; a project that wants a floor sets one (`Weights::min_similarity`, `Ranked.floor`).
2. **`0.30`.** Cost: at `k = 3` a person sees no difference and none of the 16 no-answer queries is silenced — it shortens a long list and
   nothing else. Benefit: the safest positive number (a 0.105 margin under the weakest top result in either wording).
3. **`0.35`.** Cost: a 0.055 margin; 12 of the 16 no-answer queries still answer. Benefit: the list falls from ~1 300–1 440 results to
   55–66 at ~50 % precision, 4 of 16 unanswerable descriptions are told *"no result scored high enough; N were below the floor"* — the
   sentence `Answer::describe` already says — and **every description keeps its top result in both wordings**.
4. **`0.40`.** Cost: a **0.005** margin: the weakest top result of the stranger's wording is 0.405, so a third wording, or a library a
   little less friendly than ESC-50, loses answers. Benefit: 7 of 16 silenced and lists of ~33–43.
5. **`0.45`.** The knee in the maintainer's wording (every description keeps a right answer in its first three; 9 of 16 silenced; lists of
   ~28 at 82 %). **Cost: in the stranger's wording two descriptions already get no result at all and hit@3 falls from 84 % to 80 %.** The
   number the friendly measurement recommends is the number the second measurement refutes.
6. **A per-library floor**, derived from the index itself (for example a percentile of the scores of descriptions that cannot match) so
   it adapts to the library instead of to ESC-50. Cost: a new mechanism and a new public type, and it needs the second library below to be
   designed against. Benefit: it removes the question.

## Recommendation

**Option 3, `0.35`** — and treat the number as a starting point a project overrides, not a calibration.

- The crate's rule is *fail loud*: a floor that is too **high** is not silent here — `Tally::below_floor` counts what it dropped and
  `describe()` says so — while a floor that is too **low** is exactly the silent failure (a confident-looking `cow` for a lion). That
  asymmetry favours a positive default.
- **`0.35`, not `0.45`, because the second wording changed the answer.** On the maintainer's own descriptions 0.45 looked like the knee;
  on a stranger's it already deletes answers, and 0.40 is safe by five thousandths. 0.35 is the largest value with room to spare across
  both, and it still shrinks a result list 22–24× and silences a quarter of the descriptions of sounds the library lacks.
- **Revisit when a second library exists.** Music, speech and game foley are what the set cannot say anything about; the guide names a
  second library as the most useful next addition, and option 6 should be designed against it.

If the owner would rather not change a default on one library's evidence, option 1 with the tables in the guide is defensible — the cost
is a sentence saying so, and it is the status quo.

## What is blocked

Nothing. The default stays `0.0`; `Floor::min_score`, `Weights::min_similarity` and `golden-set.md` state the measurement and point
here. A change is two lines (`Floor::default`, `Weights::default_min_similarity`), an update of the test that pins `0.0` in a defaults
test (`tests/semantic_weights.rs`), and a row in `docs/guide/semantic-config.md`. The loop did not change them.
