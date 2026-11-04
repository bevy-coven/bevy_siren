# Iteration 111 — plan: the golden set's text half (`#35`)

## Ranked

Every open issue was re-read in full this time, and the iteration-110 top five re-checked against it.

| # | item | verdict |
|---|---|---|
| 1 | **`#35` golden set — the part a machine can do:** a fixed, versioned, human-labelled library with a licence line per asset; text queries with expected answers and rationales; **the first measurement of recall, precision and the floor on real recordings**; one command to load | **chosen** |
| 2 | model-backed incremental update (`update_from_model`) | next. Ergonomics; no accuracy evidence |
| 3 | the 26 ms mel stage | later. Performance |
| 4 | `#39` music encoder | **decided, not blocked**: ADR 0005 (accepted) says *"`#39`'s music-specific encoder does not ship until a commercially-licensed one exists and is audited"* — MERT and MuQ are CC BY-NC. The issue text still says "blocked on a decision". Comment, do not close (closing a p1 is the owner's call) |
| 5 | `#32`–`#34` the editor panel | `bevy_grimoire`'s own loop builds it (`AGENTS.md`: *"the editor item waits"*); none is filed there with `consumer:bevy_siren`, so there is nothing for this repository to ship against |

**Why 1 beats 2:** `docs/guide/semantic.md` still opens with *"recall against a labelled set is still the design defaults (`V-01`). Read
the rest with that in mind"*, and `rank.rs` justifies the default floor from six synthetic clips. It is the one thing that makes every
other number in the layer a claim about something. It is also the first link of the critical path: `#35` blocks `#39` and `#45`'s real corpus.

**What is carved off (and why it cannot be built here):** `#35`'s third criterion — *"every anchor pair has a human judgement and the
reason"*, at least 50 pairs. That is a person's listening, and the issue's own scope excludes automatic labelling. This iteration
builds everything around it so that it is data entry: the format, the loader that refuses an unreasoned judgement, and the metric that
consumes it.

## Design

**`tools/fetch_esc50.py <dir>`** (written, stdlib only): downloads the **pinned commit** of ESC-50 (`33c8ce9e…`), verifies **every
file** against `golden/esc50/manifest.sha256`, refuses a stray `.wav`, writes `labels.csv` (from the dataset's own CSV, cross-checked
against `queries.tsv`) and `licences.csv` (from the dataset's `LICENSE`, a row for every clip or it fails). Idempotent: a complete
verified directory downloads nothing.

**`golden/esc50/`** (checked in, small): `manifest.sha256` (2 002 lines: the set's identity), `queries.tsv` (50 positive + 16
negative queries, each with a one-line rationale). **No audio, no labels table, no licence table** — those are fetched and verified.

**`semantic::eval`** (additive): set-valued relevance, because a class has many clips.
`ScoredCase { description, right, scored }`; `hit_at_k`, `precision_at_k`, `reciprocal_rank`, `average_precision_at_k`;
`score_cases → SetScores` (hit@1/3/10, P@10, MRR, mAP@10, the misses by name); `floor_curve → FloorRow` (what a floor keeps and what
it silences); `score_bands` (the cosines of right and wrong answers, quantiles).

**`semantic::golden`** (new, additive): `GoldenSet::parse(labels_csv, queries_tsv)` — the validation is the point:
a query with no rationale, a duplicate description, a class with no labelled file, and a header that does not name its columns are each
refused **naming the row**. `right_answers(&Library)`, `absent(&Library)` (labelled files the library lacks — a partial fetch would
change every number), `unlabelled(&Library)`.

**`examples/goldenbench.rs`** (`required-features = ["semantic"]`): index the directory with the real audio graph (or reuse a current
index), embed the queries, rank **every clip** per query with `rank::top_k`, and print:
(1) the **paper's zero-shot accuracy** from the same embeddings — the pipeline against a published number;
(2) text-to-audio **retrieval** hit@1/3/10, P@10, MRR, mAP@10, the 10 weakest classes by name;
(3) the **floor curve** and the right/wrong **score bands**;
(4) that the **tool path** (`tools::search_music`) returns the same top 10 as the raw ranking — the number is about the thing a game calls;
(5) a **baseline** compare (`--bless` records), refused when the corpus id or the model stamp differs, because *a metric compared
against a baseline from a different corpus is a number about nothing* (`evalbench`'s own rule).

## Decided and recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| the corpus | **ESC-50** | human-labelled (S2); 50 × 40; the benchmark the model's paper uses, so there is a published number to land near (S1); small (600 MB); a pinned immutable commit; **a licence per clip**. Not FSD50K (24 GB, split archives), not Clotho/AudioCaps (captions, but YouTube-sourced audio with no per-clip licence), not Kenney packs (CC0, but no stable URL and the label is a file name) |
| how it is distributed | **fetched, never vendored or packaged** | the whole set is CC BY-NC; `Cargo.toml`'s `include` does not name `golden/`'s audio (there is none in the repo) |
| integrity | a **per-file manifest** in the repo | a hash of a GitHub archive is not stable; ADR 0005's rule — *compared against a value recorded in the crate* |
| labels in the repo? | **no** | derived from NC-licensed metadata; fetched and cross-checked instead |
| relevance | the **whole class** | S1; a text encoder cannot tell twelve barks apart |
| negative queries | **16**, with the near-miss confound named in each rationale | the floor's job is an empty result; without them it can only be shown to cost recall |
| the model's training overlap | **stated, not hidden** | S1b: ESC-50 overlaps the checkpoint's training data, so absolute accuracy is optimistic; what the set measures is the pipeline's agreement with the reference and the floor's *shape* |
| changing `Floor::default()` | **not here** | a public default; if the data argue for another value it goes to `needs-decision/default-floor.md` with the table |
| `MEASURED_MATCH_FLOOR` / `MEASURED_MISS_CEILING` | **kept**, their docs say what superseded them | tests pin them; the history is part of the evidence |

## Fields

None on `CueTable` / `SirenSettings` / `MusicScore`. New public items, all additive and behind `semantic`: `eval::{ScoredCase, SetScores,
FloorRow, ScoreBand, hit_at_k, precision_at_k, reciprocal_rank, average_precision_at_k, score_cases, floor_curve, score_bands, REPORT_KS}`
and `golden::{GoldenSet, GoldenQuery, GoldenProblem}`. **Defaults and their sources:** the report's cut-offs 1 / 3 / 10 (S1 reports R@1/5/10 and
mAP@10; **3** is `#29`'s *describe, see three, pick one*, ADR 0007); the floor sweep's grid (−0.10 to 0.50 in 0.05 steps) is a **design default**
that spans S3's *"roughly zero to roughly .4"*; the zero-shot prompt is S1's.

## Tests (`tests/semantic_golden.rs`, a module of `all.rs`)

1. **The metrics, against orders a person can check by eye**: `hit_at_k` is a prefix test (rank 9 is not found by k = 3); `precision_at_k`
   with fewer than k results divides by k; `reciprocal_rank`; `average_precision_at_k` against a hand-computed 0.75 case; a case with no right
   answers is excluded from `score_cases` and is what `silent` counts.
2. **The floor curve**: a floor above every score returns nothing and silences every negative and answers no positive; a floor below
   every score returns everything; the middle row's four numbers hand-computed; a right answer that is below the floor *and* first is a loss.
3. **The bands**: quantiles of a known list.
4. **Loading refuses, naming the row**: missing rationale; duplicate description; unknown class; no header; a class with queries but no
   labelled file; both CSV spellings (`file,class` and the dataset's `filename,…,category`) read.
5. **The checked-in set's own invariants** (criteria 1 and 2 of `#35`, as a test): the manifest has 2 000 audio lines, 40 per target, 50
   targets, a 64-hex hash each, `meta/esc50.csv` and `LICENSE`; the queries cover exactly those 50 targets, with a rationale on every row,
   unique descriptions, and at least one negative.

**No test runs the fetch script or the benchmark** — CI has no network and no model. They are run, and the run is the evidence.
