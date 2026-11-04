# Iteration 111 — research: measuring the semantic layer against labels

**The question.** Every claim the semantic layer makes about *retrieval quality* rests on **six synthetic clips and five
descriptions** (`rank.rs`, `MEASURED_MATCH_FLOOR`'s own doc; `runs/2026-11-03-63/research.md` S3). `#35` is the issue whose whole job
is to replace that with a labelled set, and it gates `#39` and the real-corpus half of `#45`. Iteration 110 made the real CLAP graphs
run on this Mac, so a real measurement is possible for the first time. What is the right set, protocol and metric?

## Sources

| # | source | what it gives, and the number |
|---|---|---|
| S1 | Wu, Chen, Zhang et al., *Large-Scale Contrastive Language-Audio Pretraining with Feature Fusion and Keyword-to-Caption Augmentation*, ICASSP 2023, DOI `10.1109/icassp49357.2023.10095969` (home-still) | **The protocol and the reference number.** Zero-shot classification on ESC-50 is *audio-to-text retrieval*, one text per class, the prompt `"This a sound of {label}."`, top-1 accuracy. They report ESC-50 zero-shot of **88.0 – 91.0 %** across the CLAP variants (the OCR of the table is garbled: 89.1 for CLAP, 88.0 with fusion, 91.0 with keyword-to-caption augmentation, the last being the best model and the one `laion/clap-htsat-fused` is). Text-to-audio retrieval is reported as recall at ranks and **mAP@10** |
| S1b | the same paper, §zero-shot | *"We noticed a dataset overlap between our training data and the zero-shot dataset … We excluded all the overlap samples."* Its overlap table lists ESC-50 clips shared with Clotho (94 / 27 / 34 across train / valid / test) and with **FSD50K-train (399)**. So the published figure is on the **overlap-excluded** subset, and the *full* ESC-50 contains clips the shipped checkpoint was trained on — the absolute accuracy measured here is optimistic, and the doc must say so |
| S2 | K. J. Piczak, *ESC: Dataset for Environmental Sound Classification*, ACM Multimedia 2015, DOI `10.1145/2733373.2806390` (README of `karolpiczak/ESC-50`, read 2026-11-03) | **The corpus.** 2 000 clips, 5 s, 44.1 kHz mono, **50 classes × 40**, five folds, labels curated from Freesound recordings. **Human accuracy 81.3 %** (crowdsourced, same README) — a ceiling for what "the right class" means: people confuse these classes too. Licence: the whole set **CC BY-NC**, the ESC-10 subset CC BY, and **each clip's author and licence is listed** in the repository's `LICENSE` |
| S3 | Hessel et al., *CLIPScore*, EMNLP 2021, DOI `10.18653/v1/2021.emnlp-main.595` (home-still) | **Cross-modal cosines are small.** *"we never observed a negative cosine similarity … values ranging from roughly zero to roughly .4"* for CLIP image–text, which is why the paper rescales by 2.5. The same shape is expected of CLAP text–audio, and is the reason a floor of 0.5 — what intuition suggests — is a number about nothing |
| S4 | `docs/adr/0005-models-and-licences.md` | The weights are Apache-2.0; the corpus they were trained on has **no licence file** and includes purchased libraries. A golden set is evaluation data, never shipped: it must not make that worse, so it is **fetched, never vendored or packaged** |
| S5 | `runs/2026-11-03-110/` (this crate) | The real path exists and agrees with `transformers` to 4.9 × 10⁻⁴ on 5 × 14 pairs at **48 kHz**. Not yet covered: **44.1 kHz input** (the resampler on real audio), 5 s clips (`repeatpad` to 10 s), and any label |
| S6 | `src/semantic/rank.rs`, `docs/guide/semantic.md` "The floor" | The current evidence: right answers **0.126 – 0.429**, wrong **−0.104 – +0.150**, *"no single threshold separates them"*, default floor **0.0**. On 6 clips. This iteration confirms or overturns it |

## The technique, in the sources' terms

1. **A labelled corpus where the label is a class and a class has many clips.** A text query's right answers are *all* clips of its
   class (S1's retrieval uses the same rule); recall is therefore *any right answer within the first k*, and precision is the share
   of the first k that are right. A metric demanding one particular clip measures the corpus, not the ranking (`evalbench`'s own
   comment says the same of its synthetic clusters).
2. **Two measurements from one set of embeddings.** (a) The **paper's** zero-shot top-1 accuracy — each clip against 50 class
   prompts — tests the *pipeline* against a published number (S1). (b) **Text-to-audio retrieval** with free-text descriptions —
   each query against 2 000 clips — tests what the crate is *for*.
3. **Queries that should find nothing.** A floor exists so that a result set can be empty. Showing that it does its job needs
   descriptions of sounds the library does not contain; without them a floor can only be shown to *cost* recall. Their right
   answer is no result, and the score a *wrong* best match gets is the number a floor is chosen above.
4. **Report the failures by name** (the crate's own rule for refusals): the weakest classes, not an average.

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| zero-shot prompt | `"This is a sound of {label}."` (the paper writes "This a sound of") | S1 |
| zero-shot metric | top-1 accuracy over the class prompts | S1 |
| ESC-50 zero-shot reference | **88.0 – 91.0 %** (overlap-excluded subset) | S1 |
| retrieval cut-offs | R@1, R@5, R@10, mAP@10 (this crate reports 1, 3, 10: **3** is `#29`'s *"describe, see three, pick one"*) | S1, ADR 0007 |
| clips / classes / length / rate | 2 000 / 50 / 5 s / 44.1 kHz mono | S2 |
| human accuracy on the same labels | 81.3 % | S2 |
| cross-modal cosine range | roughly 0 – 0.4 | S3 |

## What good looks like

- **Zero-shot accuracy near the paper's 88–91 %, at or a little above it.** Above, because the full set contains training clips
  (S1b); far below (under ~80 %) would mean the pipeline is wrong, and *which* classes fail would say where.
- **Retrieval hit@3 high for descriptions that name a thing** (most of the 50), with the misses being the classes people confuse —
  `crow`/`rooster`, `rain`/`thunderstorm`, `helicopter`/`airplane`, `chainsaw`/`hand_saw`.
- **A floor curve with a visible knee**: raising the floor drops the precision of what is returned long before it drops the best
  answer, and the "nothing here" queries go silent at a floor the right answers mostly clear. If the right and wrong bands overlap
  as S6 says, the curve will show *how much*, not just that.
- **Agreement with `transformers` on 44.1 kHz audio** — the one input condition iteration 110 did not exercise.

## What this does not settle

Music (ESC-50 has none), speech, long tracks, stereo, loudness variety, and **any human judgement of "fits"**: `#35`'s anchor pairs
are a person's. The set is one library of environmental recordings and measures that library (`#35`'s own pitfall).
