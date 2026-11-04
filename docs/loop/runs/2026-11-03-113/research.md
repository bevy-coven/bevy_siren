# Iteration 113 — research: what do a game's right answers score, and can a floor tell them from nothing?

**The question.** Iteration 111 measured 2 000 environmental recordings and recommended a similarity floor of **0.35**. Iteration 112 found this
repository's own demo — fourteen macOS UI sounds, five descriptions — scoring **nothing above 0.256**, so the recommendation was withdrawn *until a
second library of game audio is measured*. This iteration measures it: a library a game would actually ship, described the way a developer would
describe it. **What does CLAP score a right answer on game sounds, and does any floor (absolute, or relative to the query) tell a description the
library can answer from one it cannot?**

## Sources

| # | source | what it gives, and the number |
|---|---|---|
| S1 | Wu, Chen, Zhang et al., *Large-Scale Contrastive Language-Audio Pretraining with Feature Fusion and Keyword-to-Caption Augmentation*, ICASSP 2023, DOI `10.1109/icassp49357.2023.10095969` (home-still; verified in iteration 111) | **The protocol and the reference.** Text-to-audio retrieval reported as R@1, R@5, R@10 and mAP@10; zero-shot ESC-50 **88.0 – 91.0 %** on the overlap-excluded subset. Its training data is a corpus of captioned recordings (LAION-Audio-630K and others): *no figure in it is about a game library*, so nothing in the paper says what a Kenney effect scores |
| S2 | Hessel et al., *CLIPScore*, EMNLP 2021, DOI `10.18653/v1/2021.emnlp-main.595` (home-still) | **Cross-modal cosines are small**: *"values ranging from roughly zero to roughly .4"* for image–text. A floor chosen by intuition (0.5) is a number about nothing; the scale is the model's and the library's |
| S3 | Bogolin, Croitoru, Jin, Liu & Albanie, *Cross Modal Retrieval with Querybank Normalisation*, CVPR 2022, DOI `10.1109/cvpr52688.2022.00513` (`paper_search`, abstract read) | **Raw similarities are not comparable across queries.** Joint embeddings suffer from *hubness* — a few gallery items are the nearest neighbour of many queries — and re-normalising a query's similarities against a bank of other queries improves retrieval *without retraining*. It normalises **against other queries**, not against the gallery's median as this iteration tries; it is the precedent for *normalising*, not evidence for the median |
| S4 | Faggioli, Formal, Lupart, Marchesin, Clinchant, Ferro & Piwowarski, *Towards Query Performance Prediction for Neural Information Retrieval: Challenges and Opportunities*, ICTIR 2023, DOI `10.1145/3578337.3605142` (`paper_search`, abstract read) | **A query's quality can be predicted from its similarity to the corpus, with no relevance judgement**: *"computing the query-corpus semantic similarity"* adapts traditional predictors to neural retrieval. A floor on the top score is exactly such a predictor, and S4 is the reason to ask whether a *normalised* one is better |
| S5 | Ma, Ai, Wu, Shao, Liu, Zhang & Ma, *Incorporating Retrieval Information into the Truncation of Ranking Lists for Better Legal Search*, SIGIR 2022, DOI `10.1145/3477495.3531998`; and a preprint, *Beyond the Largest Gap: Multi-Boundary Ranked-List Truncation for Multi-Hop Retrieval*, DOI `10.21203/rs.3.rs-10725059/v1` (home-still) | **A floor is ranked-list truncation**: *"setting a reasonable cut-off position is quite necessary"* (S5a); *"prior work uses score distributions or dynamic thresholds"* and **"the largest score gap does not reliably indicate the final evidence boundary"** (S5b) — a caution for any statistic built on gaps. Both are text retrieval; neither is audio |
| S6 | Manning, Raghavan & Schütze, *Introduction to Information Retrieval*, 2008, DOI `10.1017/cbo9780511809071` (home-still, ch. 14) | The plain statement of a floor: *"pick a value θ and if Score > θ we declare the document to be relevant, else non-relevant"* — a classifier on a score, to be judged as one (here by its AUC and its operating points) |
| S7 | The seven archives' own text and their OpenGameArt pages, read 2026-11-03 (`https://opengameart.org/content/…`; `read` on the two whose archive does not say CC0) | **The licence, the author and the grouping.** Every page lists `License(s): CC0` and `Author: Kenney`; the archives carry *"License (CC0)"* (five) or *"You may use these sounds in personal and commercial projects"* (the UI and Digital sets, whose pages say CC0). The pages count the families: **"23 card handling sounds, 19 chip handling and 12 dice handling"**, **"9 lasers, 15 phasers, 12 power-ups, 6 zaps"**, **"17 cues each in 8-bit, hit, pizzicato, sax and steel groups"**, **"5 clicks, 2 mouseclicks, 6 rollovers and 38 switches"** — a second, independent statement of the class sizes in `tools/fetch_kenney.py`'s `SIZES` |
| S8 | This repository: iteration 110's `parity.txt` (14 macOS sounds), 111's `goldenbench` runs (ESC-50), 112's format study | **The two measurements this one is set against.** ESC-50: right-answer median **0.468**, weakest top **0.492 / 0.405** (maintainer / stranger), AUC of the top score **0.963 / 0.885**, of top − tenth **0.496 / 0.461** (a coin), of top − the library's median **0.968 / 0.925** (`../2026-11-03-111/evidence/after/floor-analysis*.txt`; the last is not in the guide). macOS UI: **0.256**, the best score of five descriptions. Re-encoding alone moves hit@1 by up to **8 points** (112), so a hit@1 difference between this library and ESC-50 smaller than that says nothing |

No paper in the corpus (nine searches over two iterations) reports a CLAP or CLAP-like score on **game or synthesised sound effects**, or a
calibrated threshold for an audio–text model. That absence is why this is measured rather than cited.

## The technique

The same as iteration 111's, applied to a different library, plus one new question.

1. **The library**: 303 files in 29 classes from seven Kenney packs (CC0): foley (RPG pack, casino pack), short jingles by instrument, UI sounds,
   hardware-synthesiser effects (MP3) and sci-fi effects (Ogg). A class is **the author's own file names, grouped** (`tools/fetch_kenney.py`'s `RULES`);
   a file whose family name says nothing a person would type (`rollover`, `glitch`, `scratch`, `tone`, `pep`), the `HIT` jingles and the 38-file
   `switch` family are not in the set. The library mixes **WAV (7), MP3 (42) and Ogg Vorbis (254)**, 44.1 and 48 kHz, mono and stereo, 0.1 – 5 s —
   the first library the crate has indexed that is not one format.
2. **The questions**: one description per class in the maintainer's wording and one in a stranger's, and **eighteen that should find nothing**, shared.
3. **The measurement**: `goldenbench` as iteration 111 built it (zero-shot, hit@k, precision, MRR, mAP@10, the bands, the floor curve), run with `--set kenney`.
4. **The new question — separation.** For each query take its **top score**; ask how well that number tells a description the library can answer
   (a positive query) from one it cannot (a no-answer query): the **AUC** (the probability a random positive query's top score exceeds a random
   no-answer query's, ties half), the **weakest answerable top** and the **strongest unanswerable top** (a floor separates them exactly when the first
   exceeds the second). Then do it again on scores **centred on each query's own median** over the library (S3's motive: raw cosines are not
   comparable across queries; S4's: predict a query from its similarity to the corpus). `eval::separation` and `eval::centred`, written first and
   checked by a second implementation in Python.

## Parameters

| parameter | value | source |
|---|---|---|
| retrieval cut-offs | 1, 3, 10 | S1 (R@1/5/10, mAP@10); 3 is `#29`'s *describe, see three, pick one* (ADR 0007) |
| absolute floors tabulated | −0.10 … 0.50 in steps of 0.05 | `goldenbench`'s `FLOORS`, a **design default** spanning S2's range |
| centred floors tabulated | 0.00 … 0.40 in steps of 0.05 | **design default**: the centred scale starts at the median, so 0 is "no better than a typical clip" |
| the normaliser | the median of a query's similarities over the library, by nearest rank | **design default** (robust to the few right answers in the library; the mean and the standard deviation are tried in the scratch analysis only) |
| zero-shot prompt | `This is a sound of <class>.` with `_` → space | S1's template, **applied to a set S1 did not publish a figure for** |
| class sizes | 3 – 24 files (303 in 29 classes) | S7's page counts where the page gives them; `SIZES` otherwise |
| tolerance on a baseline | 0.005 | `goldenbench`'s `TOLERANCE` (iteration 111) |

## What good looks like

- **The index builds on all 303 files with no failure**, across three formats — otherwise nothing downstream is about this library.
- **The report is readable without the code**: which class each query found, how far above the library's typical score its top result sits, and
  *which* descriptions a floor would empty.
- **The numbers are about game audio, and say so**: the figure that matters for the default floor is the **weakest answerable top**, compared with
  ESC-50's 0.492 / 0.405 and with iteration 110's 0.256.

## A prediction, written before the model ran on this library

(Recorded in this file and committed with the queries; the run is judged against it, and a wrong prediction is reported as one.)

| # | prediction | basis |
|---|---|---|
| P1 | the right answers' **median similarity is 0.15 – 0.35**, below ESC-50's 0.468 | S8: macOS UI sounds ≤ 0.256; synthetic audio understated a real match in 111 |
| P2 | at a floor of **0.35**, `top-1 kept` is **≤ 60 %** in the maintainer's wording (ESC-50: 90 %); at 0.45 **≤ 25 %** | P1 |
| P3 | the **AUC of the top score** between answerable and unanswerable queries is **≤ 0.85** (ESC-50: 0.963 / 0.885) | no-answer queries have real neighbours (`engine` ↔ helicopter, `explosion` ↔ gunshot) |
| P4 | **centring on the query's median raises that AUC by at least 0.03** | S3, S4; ESC-50 gave **+0.005** in the maintainer's wording (0.963 → 0.968) and **+0.040** in the stranger's (0.885 → 0.925) — so this is a bet that the offset matters at least as much on a library of synthetic sounds as it did on the harder wording there |
| P5 | **hit@1 60 – 85 %, hit@3 80 – 95 %** in the maintainer's wording; hit@1 **10 – 25 points lower** in the stranger's | ESC-50 gave 90 / 100 and 72 / 84; the classes here are fewer and more confusable |
| P6 | **zero-shot accuracy with class-name prompts 55 – 85 %** | ESC-50 87.8 %; synthetic classes and invented names (`power_up`, `sci_fi_door`) |
| P7 | **no absolute floor exists** that keeps every answerable query's top result and silences every no-answer query (the weakest answerable top ≤ the strongest unanswerable top) | P3 |
| P8 | the weakest classes by hit@3 are **electronic** (`laser`, `phaser`, `electric_zap`, `power_up`) and **confusable pairs** (`leather`/`cloth`, `engine`/`thruster`) | the labels are the author's names, and the author's categories overlap acoustically |

## What this does not settle

A person's judgement of which sound *fits* an anchor (`#35`'s third criterion — still not met); whether a text encoder *should* score a synthesised
sound lower (a model question); music (the jingles are 0.3 – 1.8 s stingers, not tracks); speech (the voice-over packs are out); any library that
is not Kenney's; and the descriptions are **one person's**, twice.
