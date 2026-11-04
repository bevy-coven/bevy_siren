# The golden set

> **What this is.** Two labelled libraries — **2 000 environmental recordings** (ESC-50) and **303 game sounds** (Kenney's CC0 packs) — each with
> two sets of descriptions a developer might type, and one command that runs the real model over all of it and prints recall, precision and
> what a similarity floor keeps and silences. It exists because every number the semantic layer used to quote about *quality* came from **six
> synthetic clips** — and the first time it was run on real recordings, it overturned one of them; the second library then showed that **the
> first library's answer does not transfer**. **Two things to read before quoting a figure:** the headline retrieval numbers are for the
> *maintainer's* wording and fall sharply in a stranger's (§4 below), and [what it cannot say](#what-it-cannot-say).

`#35` asked for a labelled library, text queries with expected answers, and anchor pairs judged by a person. The first two are here, twice;
the third is a person's listening and is not.

| `#35` criterion | state |
|---|---|
| The set's contents are fixed and versioned, with a licence line per asset | **Met, for both.** `golden/esc50/manifest.sha256` pins 2 000 clips, the dataset's labels and its licence by SHA-256 at commit `33c8ce9e`; `licences.csv` is made from the dataset's own `LICENSE` and the fetch **fails** unless every clip has a row: CC0 869, CC-BY 901, CC-BY-NC 170, CC-Sampling+ 60. `golden/kenney/manifest.sha256` pins **seven archives and 303 files**, each by SHA-256, and the fetch writes `licences.csv` with **CC0 on every file** and the page it came from |
| Every text query has an expected best result and the rationale in one line | **Met, four times**: for each library two files — the maintainer's words, and a paraphrase that avoids the class names — of 66 queries (ESC-50: 50 with right answers, 16 that should find nothing) and 47 (Kenney: 29 and 18). `every_checked_in_query_names_its_answer_and_says_why_and_every_class_is_asked_for` and `every_kenney_query_names_its_answer_and_says_why_in_both_wordings` enforce them, and a test holds the second Kenney file to *none of its class's own words* |
| Every anchor pair has a human judgement and the reason | **Not met, and not a machine's to meet.** No pairs yet. [How to add them](#growing-the-set) |
| Loading the set requires one command and no manual step | **Met, for both.** `python3 tools/fetch_esc50.py <dir>`: 79 s the first time, a second to verify afterwards; `python3 tools/fetch_kenney.py <dir>`: 5 s |

## Run it

```sh
python3 tools/fetch_esc50.py /tmp/esc50            # 2 000 clips, each checked against the manifest as it arrives
python3 tools/export_clap_onnx.py /tmp/clap        # the two graphs and the tokenizer, from your Hugging Face cache
cargo run --release --features semantic --example goldenbench -- /tmp/esc50 \
    --model /tmp/clap/clap-text.onnx --vocab /tmp/clap/tokenizer.json --audio-model /tmp/clap/clap-audio.onnx \
    --model-version "$(cat /tmp/clap/clap-export.txt)"
```

The first run indexes the 2 000 clips (**211 s** on an M4 Pro CPU, 9.5 clips/s — measured while the machine's load average was 6.5, so
read it as an upper bound; `docs/guide/semantic.md` has the quiet-machine figure per clip). Later runs reuse the index and finish in
seconds. With a file missing it says which and exits **2** before loading, building or writing anything; with the library not the
set — a labelled file absent, or a file nobody labelled — it refuses by name, because a partial copy measures a different set.

![goldenbench's output on the real model, in the maintainer's wording: zero-shot accuracy 87.8 %, retrieval hit@3 100 %, the score bands, the floor table, and a baseline with no regression](../media/golden.png)

**The second library** is run the same way, with `--set kenney` and the model flags above — 303 files index in **24 s** — and prints two sections the first
did not need: **`3b`, whether a floor can tell a description the library can answer from one it cannot**, and **`4b`**, the floor table again on scores
relative to each query. `--per-query` adds one row for each description: the class it should find, the class it found first, and where its first right answer ranked.

```sh
python3 tools/fetch_kenney.py /tmp/kenney          # 303 files from 7 archives (11.9 MB), each checked against the manifest
cargo run --release --features semantic --example goldenbench -- /tmp/kenney --set kenney --per-query
```

![goldenbench --set kenney on the real model, in the maintainer's wording: zero-shot accuracy 37.3 %, hit@1 31 %, the per-description table showing an engine found first for a dog barking, the separation section (AUC 0.759, no floor exists), both floor tables, and the new baseline](../media/golden-kenney.png)

## What it measured

**1. The pipeline agrees with the reference, on audio iteration 110 could not test.** The paper's zero-shot protocol — each clip
against `"This is a sound of <class>."`, top-1 — gives **87.8 %** (1 756 of 2 000). An independent run of the same audio through
`transformers` (`audio_model(is_longer=False)`, resampled with scipy) gives **87.8 %** too: the same figure to four places, from a
different resampler, on **44.1 kHz** input that the 5 × 14 check at 48 kHz never exercised. Over all 232 000 similarities
(116 strings × 2 000 clips) the crate's differ from `transformers`' by **8 × 10⁻⁴ on average**, **0.03 at the 99.9th percentile**, and
**0.089 at worst**; a typical clip differs by 3 × 10⁻⁴ and one in a hundred by more than 0.008. The disagreement is not digital silence
(Spearman −0.21 against each clip's share of exactly-zero samples), and which of the two resamplers is nearer the one the model was
trained with — `librosa`'s — is **not measured**. It moved one top-1 of fifty (`hit@1` 0.92 in the reference, 0.90 here).

The paper reports **88.0 – 91.0 %** on ESC-50 across its variants (the table is OCR-garbled in the corpus, so which row is
`laion/clap-htsat-fused` is not certain), so 87.8 % is 0.2 points under its lowest figure. The reference's identical result puts that
gap in the checkpoint and the protocol, not in this crate. The weakest
classes are the ones people confuse: `cow` 35 % (23 of 40 read as `sheep`), `crickets` 38 % (→ `insects`), `wind` 45 %, `water_drops` 48 %
(→ `pouring_water`), `hen` 70 % (→ `rooster`). People score 81.3 % on the same labels.

**2. Retrieval, which is what the crate is for.** Fifty descriptions, every clip ranked, a right answer being any clip of the class:

| hit@1 | hit@3 | hit@10 | precision@10 | MRR | mAP@10 |
|---|---|---|---|---|---|
| 90.0 % | **100.0 %** | 100.0 % | 88.8 % | 0.950 | 0.857 |

**In the maintainer's words, every description found a right answer in the first three** (in a stranger's, 84 % did: §4). The five whose first result was wrong are `a car engine running`
(precision@10 20 %), `a cow mooing` (20 %), `a rooster crowing at dawn` (40 %), `flies and insects buzzing` (60 %) and `a hen
clucking` (70 %): a cow's moo ranks sheep first, and an engine ranks other motors. `search_music` returns the same ten, with the same
similarities, as the raw ranking for 50 of 50 queries — the numbers are about the thing a game calls.

**3. The floor, and the claim it overturned.** Until iteration 111 this guide said the right and wrong bands **overlap**, so *"no
single threshold separates them"* and *"a floor of 0.5 rejects every correct match"*. That came from six synthetic clips. On 2 000 real
ones:

| | min | p05 | p25 | median | p75 | p95 | max | n |
|---|---|---|---|---|---|---|---|---|
| right answers | −0.024 | +0.273 | +0.394 | **+0.468** | +0.536 | +0.631 | +0.794 | 2 000 |
| wrong answers | −0.444 | −0.132 | −0.013 | +0.070 | +0.154 | **+0.277** | +0.645 | 98 000 |
| best clip of a no-answer query | +0.320 | +0.325 | +0.373 | +0.442 | +0.504 | +0.530 | +0.593 | 16 |

The bands overlap only in their tails (the right answers' 5th percentile, 0.273, against the wrong answers' 95th, 0.277). The six
synthetic tones' *best* match (0.429) is below the real median: **synthetic audio understates a real match by a wide margin, and a
floor calibrated on it is calibrated on nothing.** What each floor does:

| floor | top-1 kept | hit@3 kept | results per query | precision | no-answer queries silenced |
|---|---|---|---|---|---|
| 0.00 (the default) | 90.0 % | 100 % | 1 440.3 | 2.8 % | 0 of 16 |
| 0.30 | 90.0 % | 100 % | 106.9 | 34.6 % | 0 of 16 |
| 0.40 | 90.0 % | 100 % | 42.5 | 68.6 % | 7 of 16 |
| **0.45** | 90.0 % | **100 %** | **27.7** | **81.6 %** | **9 of 16** |
| 0.50 | 90.0 % | 98 % | 16.5 | 90.4 % | 11 of 16 |

The weakest **top result** of any of the fifty descriptions scores **0.492** — and **0.405** in a stranger's words (§4). A floor on that
top score separates a description the library can answer from one it cannot with an **AUC of 0.963** (0.885 in the stranger's); a
margin over the tenth result does not (0.496 — a coin). The largest floor that keeps every description's top result in *both* wordings
is 0.40, by a margin of 0.005; **0.35 keeps it with 0.055 to spare**. That is the evidence the default floor was waiting for, and the
reason it is now a decision file rather than a number in a doc comment: `docs/loop/needs-decision/default-floor.md`. **One library is not
enough to set a default on, and iteration 112 found the repository's own counter-example**: on fourteen macOS UI sounds nothing any of
five descriptions found scored above **0.256** ("a short electronic beep" → `Sosumi` at 0.206; "a glass being tapped" → `Glass` at 0.016),
so every positive floor in this table would have returned an empty list. **The second library, measured in iteration 113 (below), settled it the same way**: the decision file recommends
keeping `0.0`, and the `0.35` this paragraph calls safe is safe on ESC-50 only.

**The sixteen that should find nothing find *something*, and it is almost always the confound their rationale names.** A gunshot finds
`fireworks` (0.593), a lion finds `cow` (0.530), a violin finds `siren` (0.529), a whale finds `wind` (0.507), a horse finds `footsteps`
(0.504). The model is not wrong — those are the nearest sounds the library has — and no absolute floor can tell *nearest* from *right*.
The ones a floor does silence are the sounds with no neighbour: an electric guitar (0.320 → `clapping`), a drum kit (0.325), a person
talking (0.331).

**4. Asked in a stranger's words, it is materially worse — and the floor's safe range shrinks.** `golden/esc50/queries-paraphrase.tsv`
describes the same 50 classes without their names (*"a furry pet yapping at the door"*, *"surf rolling up a beach"*, *"ah-choo"*) and
keeps the same 16 no-answer queries. The index and the model are unchanged; only the questions differ:

| | hit@1 | hit@3 | hit@10 | precision@10 | MRR | mAP@10 |
|---|---|---|---|---|---|---|
| the maintainer's words | 90.0 % | 100.0 % | 100.0 % | 88.8 % | 0.950 | 0.857 |
| a stranger's words | **72.0 %** | **84.0 %** | 92.0 % | 73.6 % | 0.791 | 0.676 |

Eight descriptions miss at three, and the misses name an *idea* where the model matched a *sound*: *"the loud morning call of a male
farmyard bird"* finds a right answer at rank 48, *"rockets bursting in the night sky"* at 24, *"summer evening chirping from the
grass"* at 23. The right answers' median falls from 0.468 to 0.402 and their 5th percentile from 0.273 to 0.158, so the bands overlap
more — and the floor curve moves with them:

| floor | top-1 kept (maintainer / stranger) | hit@3 kept | descriptions that still get a result |
|---|---|---|---|
| 0.00 – 0.40 | 90 % / 72 % | 100 % / 84 % | 50 / 50 |
| 0.45 | 90 % / **70 %** | 100 % / **80 %** | 50 / **48** |
| 0.50 | 90 % / **64 %** | 98 % / **70 %** | 49 / **41** |

**This is the most important caveat on every figure above.** The headline retrieval numbers describe friendly wording, and the floor of
0.45 that looked like the knee in the maintainer's wording already deletes answers in the stranger's — which is why the number the
decision file weighed was 0.35 and not 0.45, and why the second library, below, withdrew 0.35 as well.

## The second library: game audio

`#35`'s own pitfall — *a set drawn from one library measures that library* — and iteration 112's finding that this repository's own demo (fourteen macOS
UI sounds) contradicted a floor recommendation made on ESC-50 are why there is a second library, and why it is a different *kind*: **303 files in 29
classes from seven of Kenney's CC0 packs** — foley (footsteps, doors, books, cloth, knives, a pot, leather), casino sounds (cards, chips, dice), short
jingles by instrument, UI sounds, hardware-synthesiser effects and sci-fi effects — as **WAV, MP3 and Ogg Vorbis in one library**, 0.01 – 5 s, **221 of
the 303 under a second**.

**What a class is, and what is left out.** A class is **the author's own file names, grouped**: `footstep00` – `footstep09` are `footstep`; `laser1` –
`laser9` (Digital pack, MP3) and `laserLarge_000` – `laserSmall_004` (Sci-Fi pack, Ogg) are `laser`. Nothing is a listener's judgement, so a class is
only as good as Kenney's naming: `laser`, `phaser` and `electric_zap` are three of one pack page's own categories, and they are close. Left out, by a
rule written **before the model ran** and applied to names and sizes alone: families whose names say nothing a person would type (`rollover`,
`glitch`, `scratch`, `tone`, `pep` …), the 38-file `switch` family (an eighth of the library, and one class), the `HIT` jingles, singletons, and the two
voice-over packs. **That is a choice, and it can flatter the result**: it removes some of the most ambiguous sounds. The queries were written and committed
first, with a prediction (`docs/loop/runs/2026-11-03-113/research.md`).

**The result, beside ESC-50's** (maintainer's / stranger's wording):

| | ESC-50 | Kenney |
|---|---|---|
| files · classes · descriptions that should find nothing | 2 000 · 50 · 16 | 303 · 29 · 18 |
| zero-shot accuracy, class-name prompts | 87.8 % | **37.3 %** |
| hit@1 | 90 / 72 % | **31 / 31 %** |
| hit@3 | 100 / 84 % | **51.7 / 37.9 %** |
| hit@10 | 100 / 92 % | 65.5 / 48.3 % |
| mAP@10 | 0.857 / 0.676 | 0.258 / 0.209 |
| the right answers' median cosine | 0.468 / 0.402 | **0.191 / 0.193** |
| the weakest top result of an answerable description | 0.492 / 0.405 | **0.261 / 0.238** |
| the strongest top result of a description that should find nothing | 0.593 | 0.490 |
| AUC of the top score, answerable against unanswerable | 0.963 / 0.885 | **0.759 / 0.808** |
| … its 95 % interval over the descriptions | 0.90 – 1.00 / 0.78 – 0.96 | 0.59 – 0.90 / 0.65 – 0.94 |
| … on scores less each query's own median | 0.968 / 0.924 | **0.626 / 0.703** |
| answerable descriptions whose best result is under 0.35 | 0 of 50 / 0 of 50 | **13 of 29 / 6 of 29** |
| a floor that keeps every answer and silences every miss | none | none |

**How sure** (`evidence/after/bootstrap.txt`: 4 000 resamples of the *descriptions*, since they are the sample). **The AUC is not what carries the conclusion**: the two libraries' intervals are clearly apart in
the maintainer's wording (0.59 – 0.90 against 0.90 – 1.00, touching) and overlap broadly in the stranger's. What carries it is a count: **13 of 29 descriptions have a best result under 0.35 on Kenney (Wilson 95 %:
28 – 62 %) and 6 of 29 in the stranger's wording (10 – 38 %), against 0 of 50 (0 – 7 %) on ESC-50 in both.** That centring on the query's median *lowers* the AUC on Kenney holds in both wordings (−0.13 and −0.11;
the intervals of the difference, −0.25 – −0.03 and −0.22 – −0.01, exclude zero); that it *raised* ESC-50's does not (+0.005 and +0.039, intervals −0.01 – +0.02 and −0.001 – +0.09). And **0.20** costs no description
on either library in either wording — but 0 of 29 is also compatible with up to 12 % of a library's descriptions (Wilson), so it is *the largest floor that cost nothing here*, not a safe one.

**1. Right answers score less than half as much, and the strongest no-answer top beats 27 of the 29 answerable ones.** The right answers' median is
0.191 against ESC-50's 0.468, so the 0.35 that iteration 111 recommended is **above the best result of 13 of the 29 descriptions** (6 in the stranger's wording): for those it returns nothing at all.
*A helicopter flying overhead* finds an `engine` at 0.490 — higher than the best result of every answerable description but two (25 of 29 in the stranger's wording). This is iteration 110's
demo (`Sosumi` at 0.206) on a library that is not a demo.

**2. Retrieval of one-shots is weak, and it is the model's.** hit@1 is 31 %: **twenty of twenty-nine descriptions find another class first**, and *footsteps walking on the ground* finds
an `engine` (0.353) with its first footstep at rank 14. Zero-shot, **seven classes score 0 %** (`computer_noise`, `electric_zap`, `force_field`, `metal_impact`, `phaser`, `power_up`, `scrolling`) and two score
100 % (`explosion`, `thruster`); `pizzicato` → `power_up` ×13, `phaser` → `power_up` ×10 and `engine` → `force_field` ×9 are the most confused. A text description is a short list for a person here (hit@10 65.5 % and 48.3 %), not an answer.

**3. One class is the first result for almost half of all questions: a hub.** `engine` is first for **22 of the 47 descriptions** (21 in the stranger's wording, where `thruster` takes 9 more) — 8 of the 29 answerable and 14 of the 18 that
should find nothing, among them *a dog barking* (0.216) and *a person speaking* (0.204). **Thirty distinct files supply all 47 first results** (28 in the stranger's), 238 of the 303 files are in no description's
top three, and the skewness of that count (the usual measure of hubness) is 4.78 (4.90 in the stranger's wording) against ESC-50's 3.82 (3.65). A hub is a file that is near everything, so its cosine
says little about the description — which is why no cosine threshold separates the two kinds of description here.

**4. The floor does not transfer.** What each floor keeps and silences on this library (maintainer's / stranger's):

| floor | top-1 kept | hit@3 kept | results per query | no-answer queries silenced |
|---|---|---|---|---|
| 0.00 (the default) | 31.0 / 31.0 % | 51.7 / 37.9 % | 133 / 140 | 0 of 18 |
| 0.20 | 31.0 / 31.0 % | 51.7 / 37.9 % | 23 / 24 | 0 |
| 0.25 | 31.0 / 27.6 % | 51.7 / 34.5 % | 14 / 15 | 4 |
| 0.30 | 31.0 / 27.6 % | **44.8** / 34.5 % | 7 / 9 | 9 |
| 0.35 | **20.7 / 24.1 %** | **27.6 / 31.0 %** | 3 / 4 | 15 |
| 0.40 | **17.2 / 13.8 %** | **24.1 / 13.8 %** | 1 / 2 | 15 |

Iteration 111's `0.35` — free on ESC-50 — would have deleted 10 points of the maintainer's top-1 here and 24 of its hit@3. **The largest floor that costs neither library an answer in either
wording is 0.20, and at 0.20 it silences none of the no-answer queries on either**: a floor high enough to silence half of them (0.30) deletes answers, and one that silences most (0.35) deletes between a fifth and a half of the hits.

**5. A floor relative to the query is worse here, not better — the offset is not in the query.** Centring each query's scores on its own median (`eval::centred`) lowers the AUC from 0.759 to
0.626 (0.808 to 0.703), where on ESC-50 it made no clear difference (0.963 → 0.968; 0.885 → 0.924, whose interval touches zero). And the floor table on those scores (`4b`) is no kinder: at 0.30 it keeps 27.6 % of the top results and silences 28 % of the no-answer queries.

**6. What does help is on the other side, and is exploratory.** Subtracting from each *file's* score its mean similarity to the *other* descriptions of the set — Querybank Normalisation's idea (Bogolin et
al., CVPR 2022, DOI `10.1109/cvpr52688.2022.00513`) in its simplest form — changes, in Python on the crate's own similarities (`evidence/after/normalisations.py`):

| | hit@1 | hit@3 | AUC of the top score |
|---|---|---|---|
| Kenney, maintainer's | 31.0 → **44.8 %** | 51.7 → **62.1 %** | 0.759 → **0.925** |
| Kenney, stranger's | 31.0 → 31.0 % | 37.9 → **51.7 %** | 0.808 → **0.910** |
| ESC-50, maintainer's | 90 → **100 %** | 100 → 100 % | 0.963 → 0.969 |
| ESC-50, stranger's | 72 → **80 %** | 84 → 84 % | 0.885 → **0.939** |

It helps both libraries, most where the hub is worst. **Not built, and not proved**: the bank here is the set's own other descriptions (so it knows the kind of question that will be asked — a bank of unrelated
text may behave differently), there are 29 or 50 queries, and **no floor exists even then** (Kenney: the weakest answerable top is +0.183, the strongest unanswerable +0.342). It is the most promising direction
this measurement found, and it needs an index-time statistic and a bank — a public-format change, so it is in `docs/loop/needs-decision/default-floor.md`, not in the crate.

**The prediction, scored** (written before the model ran; `research.md`). Right: the right answers' median is **0.191** (predicted 0.15 – 0.35); at 0.35 `top-1 kept` is **20.7 %** (≤ 60 %); the AUC is **0.759**
(≤ 0.85); no floor exists. **Wrong**: centring on the query's median was predicted to *raise* the AUC by 0.03 and lowered it by 0.13; hit@1 was predicted at 60 – 85 % and hit@3 at 80 – 95 % and are **31 %** and **51.7 %**;
the stranger's wording was predicted 10 – 25 points lower and is **equal** at hit@1; zero-shot was predicted at 55 – 85 % and is **37.3 %**. **Half right**: the weakest classes were predicted to be the electronic
ones and two confusable pairs — `electric_zap` and `phaser` score 0 %, but so do `computer_noise`, `force_field` and `metal_impact`, and `leather`/`cloth` and `engine`/`thruster` were not the weakest. The predictions were
optimistic about the model, not about the pipeline.

**Two checks that these are the model's numbers and not a defect.** *The metric code*: every figure the report prints — the seven metrics, the separation, 55 cells of each floor table — is recomputed in Python from
the crate's scores and **agrees to the printed precision on both libraries and both wordings** (`evidence/after/kenney-parity.txt`). *The model path*: against `transformers` on the same files (libsndfile for Ogg and MP3,
scipy for resampling), 23 028 similarities differ by **0.004 on average**; the 39 files at 48 kHz are **identical** (median per-clip difference 1.8 × 10⁻⁶), the 264 at 44.1 kHz differ by a median 1.1 × 10⁻³ and by up to
0.049 for a clip and **0.213 for one pair** — a resampler against a resampler, not the decoder (iteration 112) or the length (clips under a second differ no more than longer ones). The retrieval metrics agree: hit@1 and hit@3
exactly, hit@10 within one query, zero-shot within two clips. Which of the two resamplers is nearer the one the model was trained with is **not measured**.

**One thing that is not the explanation: how a short clip is padded.** The model's feature extractor fills its 10 s window by *tiling* a shorter clip — a 0.28 s footstep becomes thirty-five footsteps in a row — and the
crate reproduces that exactly. Embedding the same files with the clip once and then silence (not what the model was trained with; run to find out, not to change anything) moved the median right-answer cosine of the 221 clips
under a second from +0.162 to +0.233 in the maintainer's wording and from +0.175 to +0.181 in the stranger's, the AUC from 0.747 to 0.874 in one wording and 0.805 to 0.803 in the other, and hit@1 from 0.310 to 0.276 in both.
No consistent gain, so no change: the crate keeps the model's own padding.

## What it cannot say

- **ESC-50 overlaps the model's training data.** The paper's own overlap table lists ESC-50 clips shared with Clotho and with FSD50K
  (399 in its training split alone) and the paper *excluded* them from its zero-shot figure. This set does not, so absolute scores
  here are optimistic for the shipped checkpoint. What it measures soundly is **agreement with the reference** and the **shape** of the
  floor curve; what it cannot is how a *new* sound will score.
- **The baseline is the WAV encoding, and the encoding moves the answer.** The same 2 000 clips as FLAC give identical numbers (all 232 000
  similarities equal); as MP3 they move hit@1 from 90 % to **94 %** at 128 kbit/s and **98 %** at 64, zero-shot from 87.8 % to 87.5 % and 87.2 %,
  and the stranger's hit@10 from 92 % to 88 % at 64 (`docs/guide/semantic.md` §2b, `evidence/after/format-cost.txt`). So a change in hit@1 smaller than about
  eight points — four of fifty queries — is within what *re-encoding the library* does, and is not evidence about a pipeline change. (ESC-50's own
  `LICENSE` lists all 2 000 clips as `.ogg` files, so the WAVs are themselves decoded from a lossy original.)
- **Two libraries, and neither is every library.** ESC-50 is environmental recordings; Kenney's packs are **one author's** game effects, mostly one-shots (221 of 303 under a second), in a recording style of their own.
  Neither has speech, long music tracks or ambience beds, and *the second library's answer to the floor is a number about these packs*. `#35`'s own pitfall — *a set drawn from one library measures that library* — now
  applies to two of them.
- **The second library's labels are file names.** A class is the author's grouping of his own names, so a confusable pair (`laser`/`phaser`/`electric_zap`) is a pair a developer browsing the pack would see too, and some
  of what looks like a model's error is the label's. The rule that left families out was fixed before the model ran, and **it can flatter the result**.
- **The descriptions were written by one person**, twice, with the class list in view for the first wording. Fifty queries move in steps of 2 points and twenty-nine in steps of 3.4, and a second wording by the same
  hand moved ESC-50's hit@3 from 100 % to 84 % (§4): a third writer would move it again.
- **The no-answer queries are sixteen and eighteen, and several have a real neighbour.** `silent` is a lower bound on what a floor can do and a coarse one, and on the second library the neighbour is usually the same `engine`.
- **Nothing here judges whether two sounds *fit*.** Right means *same class*; whether a neighbour belongs beside an anchor — `#39`'s
  question and `#35`'s third criterion — is a person's call.

## Growing the set

**A text query**: add a row to `golden/esc50/queries.tsv` — `target`, `class`, `description`, `rationale`. Descriptions are unique (the
loader refuses a repeat, naming both lines), a row without a rationale is refused, and a class no file is labelled with is refused rather
than blamed on the model. **Never edit a recorded query in place**: add a new row, because the baseline is about the old one.

**A query that should find nothing**: `target` and `class` of `-`, and a rationale that names the nearest confound (*"`fireworks` is the
nearest impulsive bang"*) so a reader of a result knows what the model found instead.

**A second wording** is the cheapest way to test whether a number is robust: describe the same classes without their names
(`queries-paraphrase.tsv` is the model) and run `goldenbench <dir> --queries <file>`. The baseline is for the default queries only, so
another file is measured and compared with nothing, and `--bless` refuses it.

**A third library**: a directory `golden/<name>/` holding `manifest.sha256` (pinning its contents by a `# commit:` or a `# version:`, one SHA-256 a file), `queries.tsv` (and a `queries-paraphrase.tsv`) and
`baseline.txt` (`goldenbench --set <name> --bless`), and `tools/fetch_<name>.py` — the command the benchmark names — which writes the library's `labels.csv` and `licences.csv`. `tools/fetch_kenney.py` is the model:
it checks every archive and every file against a hash recorded here and refuses a stray file by name. A test fails for a set that has any of those missing. **What is still not in either library**: speech, long
music, ambience beds and a person's judgement of *fit*.

**Anchor pairs — the part a person does.** For each of at least ten anchors: list its nearest neighbours (`examples/audition` prints them
in rank order with their scores), listen to each beside the anchor (`afplay`), and record one line per pair —
`anchor<TAB>neighbour<TAB>fits | does not fit<TAB>why` — with paths relative to the library. A pair with no reason is not a judgement.
`#35` asks for at least 50. **The loader and the metric are not built**: `eval::agreement` counts affirmed neighbours ranked above the
anchor, which an anchor queried with itself can never score (`evalbench`'s first run found that), so the right metric for real pairs is a
decision to make with the first real judgements in hand, not before.

## Licence and integrity

The dataset as a whole is **CC BY-NC 3.0**; its ESC-10 subset CC BY 3.0; each clip keeps its Freesound licence. So it is **fetched, never
vendored or packaged** (`Cargo.toml`'s `include` does not name `golden/`'s audio, and the repository holds none). The manifest records a
SHA-256 per file, compared against a value *in this repository* — ADR 0005's rule that a hash delivered beside the payload verifies nothing —
and a downloaded file that disagrees is refused by name and never written. A file that is in the directory and not in the set is refused
too, because it would join every ranking as a wrong answer. Two quirks of the dataset's own `LICENSE` were found by this check and are
named in `tools/fetch_esc50.py` rather than papered over: one clip is listed as `5-141683-Ad` (a typo for `5-141683-A`, and the row's
Freesound id is checked against the clip's), and one title is written `[DrinkingCan[Norberd]](url)`.

**What the crate makes of the real sidecar.** Joined into the index, the 2 000 licence lines read as **1 770 that permit commercial use**
(CC0 869, CC-BY 901), **170 that do not** (CC-BY-NC, by each clip's own Freesound licence) and **60 unknown** (CC-Sampling+, which
the crate does not recognise and so never presents as permissive) — the distinction `#14` exists to keep, from real data rather than a
fixture. `goldenbench` prints that line on every run.

**The second library.** Kenney's packs are **CC0** — each pack's OpenGameArt page lists it and the author, and the archives say *"License (CC0)"* or *"personal and commercial projects"* — so they could be vendored,
and are not: 12 MB the repository does not need, and the manifest makes the copy exact. `tools/fetch_kenney.py` fetches OpenGameArt's static file URLs (`kenney.nl`'s own link answers a script with 403, and that is not
worked around), checks **each archive** against a SHA-256 recorded in this repository before a byte of it is used and **each extracted file** against its own line, and refuses a stray file by name. Exercised: a complete
library, a damaged file and a damaged cached archive (both repaired), a stray file and labels that disagree with the manifest (refused), and an archive that downloads with the wrong bytes (refused, nothing written).
The crate reads **303 files that permit commercial use, 0 that do not, 0 unknown** from the sidecar.
