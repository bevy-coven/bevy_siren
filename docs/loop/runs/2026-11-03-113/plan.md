# Iteration 113 — plan: the second golden library, and the question a floor asks

## Ranked

Every open issue was re-read through `issue://` (`#32`–`#34` the editor, `#35` open on a person's anchor judgements, `#39` decided not to ship by ADR 0005;
**no `consumer` issue is open**), the baseline is green, and iteration 112's top five was re-read against what the code does:

| # | item | verdict |
|---|---|---|
| 1 | **A second golden library — game audio — and the floor measured on it** | **chosen.** It is the evidence `needs-decision/default-floor.md` is waiting on, it tests the one caveat iteration 112 left on every figure (*"one library of environmental recordings"*), and it is the first real library that is not one format (WAV, MP3 and Ogg in one run) |
| 2 | the default-floor decision itself | the owner's; item 1 decides what to recommend |
| 3 | resampler fidelity; a model-backed `update_from_model` | later — they do not change what a floor should be |
| 4 | the mixer path: *listen* to Ogg, FLAC, MP3; whether `MixerBackend` should cut the Ogg padding too | later — nobody can listen here, and the padding is −69 dBFS |
| 5 | waiting on a person, another repository or hardware: `#35`'s anchor pairs, the editor (`#32`–`#34`, pinned 216 commits back), `#39`, `#36`'s GPU figure | not buildable here |

**Why 1 beats 4 and 3:** a floor is a number every consumer inherits, and the repository holds two statements about it that disagree — `0.35` (iteration 111) and
*nothing above 0.256* (110's demo). A third measurement that is a *different kind of library* is the only thing that can say which statement is about the
crate and which about a corpus. The claims audit (`eval.md`) found nothing that outranks it: no refusal that should not happen, no claim a test does not hold.

## The design

**Data** (committed first, and frozen — the queries were written *before the model ran*; `research.md` carries the predictions):

- `tools/fetch_kenney.py` — the one command: seven CC0 archives from OpenGameArt (**11.9 MB**; `kenney.nl`'s own link refuses scripts and is not worked around), each
  checked against a SHA-256 *recorded in this repository*, then each extracted file against its own line; stray and damaged files refused by name. Exercised:
  a complete library (second run, no download), a damaged file and a damaged cached archive (both repaired), a stray file and a labels file that disagrees with the
  manifest (refused, exit 1), and an archive that downloads with the wrong bytes (refused, nothing written).
- `golden/kenney/{manifest.sha256, labels.csv, queries.tsv, queries-paraphrase.tsv}` — **303 files in 29 classes**, 47 queries a wording: 29 with right answers and 18 that
  should find nothing.
- **The class rule, fixed before any measurement.** A class is the author's own file names, grouped: `footstep00`–`09` are `footstep`; `laser1`–`9` (Digital pack, MP3) and
  `laserLarge/Retro/Small_000`–`004` (Sci-fi pack, Ogg) are `laser`. Left out: families whose names say nothing a person would type (`rollover`, `glitch`, `scratch`, `tone`,
  `pep`, `open`, `close`, `drop`…), the `HIT` jingles, the two voice-over packs, singletons (`chop`, `metalClick`, `metalLatch`, `slime`), *and the 38-file `switch` family*, which would be
  an eighth of the library and a single class. **That is a choice, made on names and sizes alone; it can flatter the result** (it removes some of the most ambiguous sounds), and the
  guide says so beside the numbers.
- Sizes, from the tool's rules, cross-checked against the pages' own counts where they give them: footstep 10, door 6, book 8, cloth 4, knife 5, metal_pot 3, leather 3,
  playing_cards 23, poker_chips 19, dice 12, chiptune 17, saxophone 17, steel_drum 17, pizzicato 17, mouse_click 12, error_beep 8, confirmation_chime 4, scrolling 5, laser 24, power_up 12,
  electric_zap 6, phaser 15, explosion 7, force_field 5, sci_fi_door 6, engine 24, thruster 5, computer_noise 4, metal_impact 5.

**Code** (every new behaviour failing first):

1. `semantic::eval::centred(cases) -> Vec<ScoredCase>` — every score less its query's own median (nearest rank, `ScoreBand`'s rule). The order and the right answers are unchanged, so
   `floor_curve` and `score_bands` apply to it as they are: the *relative floor curve* is the existing curve on centred cases.
2. `semantic::eval::separation(cases) -> Option<Separation>` — `{ answerable, unanswerable, auc, weakest_answerable, strongest_unanswerable }` over the **top score** of each query, ties
   counting half; `None` when either group is empty. `Separation::floor_exists()` is `weakest_answerable > strongest_unanswerable`.
   *Additive public items; no existing item changes* (the precedent is iteration 111's `floor_curve` and `score_bands`), so no decision file — **but any recommendation to give `Floor` a
   relative mode is a public-type change and goes to `needs-decision/`, not into the crate**.
3. `examples/goldenbench`: `--set <name>` (default `esc50`) selects `golden/<name>/{manifest.sha256, queries.tsv, baseline.txt}` and `tools/fetch_<name>.py`; the set's identity keeps ESC-50's
   string **byte-identical** (`esc50 @ 33c8ce9e manifest … queries …`) so its recorded baseline still compares, and a manifest with `# version:` instead of `# commit:` (Kenney's) reads
   `kenney @ v1 manifest … queries …`. New output: the **separation** section (raw and centred), the **centred floor table**, the **precision@10 ceiling** (a class of 3 files cannot score
   above 30 %, so a precision figure is read against what the class sizes allow), and `--per-query` (one row per description: what it found first, its top score, the rank and score of
   its first right answer). The paper's figure is printed only for ESC-50, the one set it is about.
4. The ESC-50 refactor is proved on ESC-50: `goldenbench /tmp/esc50` must still compare *"no regression"* against `golden/esc50/baseline.txt` with the same identity.

**Tests** (`tests/semantic_golden.rs`, a module of `tests/all.rs`):

- `centred`: a hand-made case's scores less the median it has, order and right answers unchanged; a case with NaN scores centres on the rest.
- `separation`: hand-computed AUC with a tie counted half (5/6 and 4.5/6 on three positives and two negatives), the weakest and strongest, `None` for an empty group, NaN tops ignored.
- the checked-in Kenney files: the manifest pins 303 audio files and seven archives; the labels and the manifest list the same files; every class has at least 3 files and the sizes the tool asserts;
  **both query files** ask for every class, give every row a one-line rationale, have 18 no-answer queries, and agree on the class names; **the stranger's wording contains none of its class's own
  words** (a name's parts of four letters or more) — the principle of the second file, enforced rather than hoped for;
- every `golden/<name>/` directory has what `--set <name>` needs (a manifest, queries, and its `tools/fetch_<name>.py`), so a set without its fetch tool is a failing test.

**Measured, not tested** (CI has no model and no network): `goldenbench --set kenney` with both query files; `--dump` cross-checked by an independent Python implementation of every metric the
new section prints; ESC-50 re-run against its baseline.

## Decisions recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| ship `centred`/`separation` as public items, or keep them in the example | **public, additive** | the tests that prove them must reach them, and the example's *"tool path"* check shows the benchmark should call the crate's own functions |
| normalise by the median, the mean, or the mean and standard deviation | **the median** in the crate; the others in the scratch analysis only | robust; and the crate should not grow three statistics to answer one question |
| zero-shot for a set with no published figure | **print it, labelled as this crate's own** | cheap, and it shows how far class-name prompts reach on invented names |
| a per-class cap to equalise class sizes | **no** | the floor question is per-query; class size only shapes the right-answer *band*, which the guide reads per query as well |
| vendor the audio (it is CC0) | **no** | 12 MB the repository does not need; the manifest makes the copy exact |

## What would change the recommendation

- If the weakest answerable top on this library is **under about 0.25**, no positive absolute floor from ESC-50 survives, and the recommendation is `0.0` or a relative floor.
- If **centred** separation is clearly better on **both** libraries (the AUC up by at least 0.03 and a threshold that keeps every answerable top and silences a real share of the
  unanswerable), the decision file recommends designing a relative floor (a new public type — the owner's call).
- If neither, the honest recommendation is `0.0`, and the guide says a floor is a per-library number a project sets from its own no-answer queries.

## Carved off

Anchor pairs (a person's); speech and long music tracks; a per-class zero-shot breakdown beyond the weakest five; moving the editor's pin.
