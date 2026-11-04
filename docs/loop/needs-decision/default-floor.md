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

## Options

1. **Keep `0.0`.** Cost: a description the library cannot answer returns `k` results that look like answers (`a lion roaring` → `cow`,
   similarity 0.53); an agent or a panel cannot tell, and the list a caller asks for with a large `k` is 97 % wrong. Benefit: no behaviour
   change, no risk from library shift; a project that wants a floor sets one (`Weights::min_similarity`, `Ranked.floor`).
2. **`0.30`.** Cost: at `k = 3` a person sees no difference and none of the 16 no-answer queries is silenced — it shortens a long list and
   nothing else. Benefit: the safest positive number (a 0.105 margin under the weakest top result in either wording).
3. **`0.35`.** Cost: a 0.055 margin on ESC-50 — and **the 14-UI-sound demo's right answer (0.206) would be silenced**; 12 of the 16 no-answer
   queries still answer. Benefit: the list falls from ~1 300–1 440 results to 55–66 at ~50 % precision, 4 of 16 unanswerable descriptions
   are told *"no result scored high enough; N were below the floor"* — the sentence `Answer::describe` already says — and **every
   description keeps its top result in both wordings** *on ESC-50*.
4. **`0.40`.** Cost: a **0.005** margin: the weakest top result of the stranger's wording is 0.405, so a third wording, or a library a
   little less friendly than ESC-50, loses answers. Benefit: 7 of 16 silenced and lists of ~33–43.
5. **`0.45`.** The knee in the maintainer's wording (every description keeps a right answer in its first three; 9 of 16 silenced; lists of
   ~28 at 82 %). **Cost: in the stranger's wording two descriptions already get no result at all and hit@3 falls from 84 % to 80 %.** The
   number the friendly measurement recommends is the number the second measurement refutes.
6. **A per-library floor**, derived from the index itself (for example a percentile of the scores of descriptions that cannot match) so
   it adapts to the library instead of to ESC-50. Cost: a new mechanism and a new public type, and it needs the second library below to be
   designed against. Benefit: it removes the question.

## Recommendation

**Revised in iteration 112: keep `0.0` for now (option 1), and decide when the second library has been measured — the next iteration's
work.** Iteration 111 recommended `0.35`. That number still holds for a library like ESC-50 and the table above has not changed; what
changed is that ESC-50 was the only library measured, and the repository's other one (fourteen macOS UI sounds, above) puts a correct top
result at **0.206**.

- **A floor that is too high is also a silent failure** for the library it silences: `Tally::below_floor` and `describe()` do say what was
  dropped, but a game that never reads them sees an empty list. A default every consumer inherits has to survive the libraries the crate
  was built for, and a game's UI sounds and synthesised effects are first among them.
- **What would settle it:** a second library of game audio (UI, foley, sci-fi, jingles — `docs/loop/runs/2026-11-03-112/plan.md` names the
  source, Kenney's CC0 packs, and the classes) with its own right-answer scores. If those sit near 0.2, the answer is option 6 (a floor
  relative to the library) or no positive default; if they sit near ESC-50's, `0.35` stands, and nothing above is lost by waiting.
- **If the owner would rather decide now, option 1 is safer than option 3.** The cost of `0.0` is a long list that looks like answers,
  and the guide says so; the cost of `0.35` is an empty one for a kind of library this crate exists for.
- **Why not `0.45`, if `0.35` is withdrawn?** Unchanged: on the stranger's wording `0.45` already deletes answers on ESC-50 itself.

## What is blocked

Nothing. The default stays `0.0`; `Floor::min_score`, `Weights::min_similarity` and `golden-set.md` state the measurement and point
here. A change is two lines (`Floor::default`, `Weights::default_min_similarity`), an update of the test that pins `0.0` in a defaults
test (`tests/semantic_weights.rs`), and a row in `docs/guide/semantic-config.md`. The loop did not change them.
