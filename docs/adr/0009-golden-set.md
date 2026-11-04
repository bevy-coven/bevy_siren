# 0009 — The golden set: a human-labelled corpus, fetched and verified, never vendored

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#35](https://github.com/bevy-coven/bevy_siren/issues/35)
- **Supersedes:** nothing. **Depends on:** [0005](0005-models-and-licences.md) (what may ship), [0004](0004-vector-store.md) (the index it measures)
- **Evidence:** [`loop/runs/2026-11-03-111/`](../loop/runs/2026-11-03-111/)

## The question

`#35` asks for a labelled library, text queries with expected answers, and anchor pairs judged by a person, so that *"every ranking
change is otherwise unmeasurable"*. Which corpus; how does it reach a developer without becoming a licence problem; how is it kept the
same between two people's machines; and what does a "right answer" mean when a class has forty clips?

## The decision

1. **The corpus is ESC-50**: 2 000 five-second recordings, 50 classes of 40, labelled by the dataset's curators from Freesound.
   It is the benchmark the model's own paper reports on (88.0 – 91.0 % zero-shot), so a published number exists to land near; it is
   small enough to fetch in a minute; and it ships **a licence per clip**.
2. **It is fetched, never vendored or packaged.** The dataset is CC BY-NC as a whole (ESC-10 is CC BY; each clip keeps its own
   Freesound licence). The repository holds no audio and no table derived from the dataset's metadata; `Cargo.toml`'s `include` does
   not name `golden/`. A project that cannot take an NC dataset into its evaluation does not run it; nothing else is affected.
3. **Integrity is a per-file manifest in this repository**, SHA-256 at a pinned commit — not a hash of a downloaded archive, because
   GitHub's archive bytes are not promised stable (and the archive route proved to be two hours at the rate GitHub streams it, where
   the files take 79 seconds). This is ADR 0005's rule applied to data: *a hash is compared against a value recorded in the crate,
   never one delivered beside the payload.* A file that disagrees is refused by name and never written; a `.wav` that is not in the
   manifest is refused too, because it would join every ranking and change every number.
4. **A right answer is any clip of the class**, and some queries have none. A text encoder cannot tell twelve takes of one bark apart,
   so a metric that demanded a particular clip would measure the corpus's order. And a floor can only be shown to *work* by descriptions
   of sounds the library does not contain, so the set carries sixteen of them, each naming its nearest confound.
5. **Labels and licences are made at fetch time from the dataset's own files** (`labels.csv`, `licences.csv`), cross-checked against the
   checked-in queries — so a mislabel in one place cannot survive — and every clip must have a licence row or the fetch fails.
6. **Human anchor judgements are not automated.** `#35` excludes automatic labelling, and *same class* is not *fits beside*. What is
   built is everything around them; the loader and the metric wait for the first real judgements, because `eval::agreement`, as written,
   cannot score an anchor queried with itself.

## Alternatives rejected

| | why not |
|---|---|
| FSD50K | 24 GB in split archives, with per-clip licences that include CC BY-NC — the cost of ESC-50's problem at forty times the size |
| Clotho / AudioCaps | human captions, but the audio is YouTube-sourced and its per-clip licences are not recorded |
| Kenney packs (CC0) | a clean licence, but no stable URL and the "label" is a file name |
| the macOS system sounds | fourteen files under Apple's licence; not redistributable and too few to measure anything |
| recordings of our own | none exist, and a set the maintainer recorded and labelled is the maintainer's opinion twice |

## What this costs, and what it cannot say

- **ESC-50 overlaps the model's training data**: the paper's overlap table lists ESC-50 clips shared with Clotho and FSD50K (399 in
  FSD50K's training split alone) and excludes them from its own figure. Absolute scores are optimistic; the agreement with the
  reference and the *shape* of the floor curve are not.
- **One domain.** No music, speech, long tracks, stereo or game foley. `#35`'s own pitfall — *a set drawn from one library measures
  that library* — is stated in the guide beside every figure, and a second library is the most useful next addition.
- **A fetch of ~880 MB** the first time, and a dataset whose own `LICENSE` has two quirks (one mistyped clip name, one malformed title)
  that the fetch names rather than papers over.
- **It overturned a documented claim on its first run**: the six-synthetic-clip evidence behind the default similarity floor. That is
  the case for having it; the consequence is `docs/loop/needs-decision/default-floor.md`.
- **Wording moves every figure.** A second file of descriptions — the same classes written without their names — drops hit@3 from 100 %
  to 84 % and the weakest top result from 0.492 to 0.405. The set therefore carries two query files, and any single number quoted from
  it is for one writer: a floor chosen on the first (0.45) would have deleted answers in the second, and the recommendation is 0.35.
