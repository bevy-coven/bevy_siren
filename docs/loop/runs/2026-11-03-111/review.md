# Iteration 111 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest
pass it could affect.

## Pass 1 — Function

*Against `#35`'s four criteria and against an independent reference; no cue-table case moves (the 33 cases of `cues.yaml` are asserted by
the matrix, unchanged, and the 21 rendered WAVs are byte-identical to iteration 110's).*

**Critique, first reading — four things that were wrong:**

1. **A prediction I wrote down before measuring, and the measurement did not oblige.** `research.md` said zero-shot accuracy should land
   *"at or a little above"* the paper's 88.0 – 91.0 %, because the full set contains clips the model trained on. It is **87.8 %** — under.
   Not massaged: an independent `transformers` run on the same audio gets 87.8 % too, so the gap to the paper is the checkpoint and the
   protocol, not this crate, and the guide says so.
2. **The headline retrieval claim was friendlier than the evidence.** The first draft of the guide, the README, the CHANGELOG and five doc
   comments said *"every description finds a right answer in its first three"* and recommended a floor of 0.45. Both were measured on the
   descriptions I wrote. Pass 3's second wording refuted them (below); every statement is now qualified, in about a dozen places.
3. **The guide said one clip was CC-BY-NC.** The crate's provenance, run on the real sidecar, says **170**. The count came from the output
   of a line added to the benchmark for that purpose (`1 770 permit commercial use, 170 do not, 60 unknown`), not from the sentence.
4. **"Reproduces `transformers` to four places"** was true of the accuracy and not of the similarities, which differ by up to 0.089
   (resampler against resampler). The README and guide now state both figures.

**Criteria, as met:**

| `#35` criterion | evidence |
|---|---|
| contents fixed and versioned, a licence line per asset | `golden/esc50/manifest.sha256`; `fetch-repair.txt` (a damaged copy is repaired, a padded one refused); the fetch **fails** unless every clip has a licence row, and found two quirks in the dataset's own `LICENSE` that way |
| every text query has an expected best result and a one-line rationale | `tests/semantic_golden.rs::every_checked_in_query_names_its_answer_and_says_why_…` over **both** query files |
| every anchor pair judged by a person, with a reason | **not met, and said so** in the benchmark's own output (section 6), the guide, the log and the issue |
| one command, no manual step | `python3 tools/fetch_esc50.py <dir>`: 79 s cold, 1 s verified |

**Against the reference** (`evidence/after/golden-parity.txt`, all 2 000 clips at 44.1 kHz): zero-shot **0.8780 / 0.8780**; the Python
metric implementation and the Rust one agree on all seven metrics to the printed precision (largest difference 3 × 10⁻⁴, a rounding);
232 000 similarities differ by 8 × 10⁻⁴ on average. The control that matters: the Rust metric code was *written first and checked by a
second implementation*, not the other way round.

**Verdict, first reading: NOT YET** (items 2 – 4 were claims that outran their evidence). **After the fixes: GREAT.**

## Pass 2 — Zoom

*Files read: `docs/media/golden.png` whole, then two 2× crops (`evidence/spectrograms/golden-zoom-top.png`, `…-floor.png`); and, for the
render gate, `evidence/spectrograms/mix.png` and `footstep-grass-spectrum.png`.*

**Critique.** The still is legible at 100 % and sharper at 2×; the colours separate the command from the output. Three defects:

1. **The baseline table printed `-0.0000`** for a difference of 10⁻⁸ — a minus sign on a zero. Rounded to zero before printing.
2. **The floor table explained four of its six columns.** `top-1 kept` and `hit@3 kept` were defined only in the guide; the footer of the
   table now defines all six, and the still was regenerated.
3. **The picture shows the maintainer's wording only, and its caption said "retrieval hit@3 100 %" without saying whose.** The guide's
   alt text and the media README now say so and point to the stranger's figures.

The audio gate: the master (`mix.png`) is three sustained tonal lines under 1 kHz and one broadband onset in the first 40 ms — no click,
no clipping streak, no hole; the footstep (`footstep-grass-spectrum.png`) is a 27 ms broadband attack ending on a sharp edge, a harmonic
series and a tail that decays to the noise floor. Both are **byte-identical** to iteration 110's WAVs (`cmp`, 21 of 21), so they can
only look the way they did. Master **−21.5 LUFS, −7.5 dBFS true peak**, unchanged.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*A different question set, then every way to hold the tool wrong.*

**The different set: the same 50 classes described the way a stranger would** (`golden/esc50/queries-paraphrase.tsv`, no class names:
*"a furry pet yapping at the door"*, *"ah-choo"*). Same index, same model. It **changed the conclusion**:

| | hit@1 | hit@3 | hit@10 | P@10 | MRR | mAP@10 | weakest top result | AUC of a floor |
|---|---|---|---|---|---|---|---|---|
| the maintainer's words | 90 % | 100 % | 100 % | 88.8 % | 0.950 | 0.857 | 0.492 | 0.963 |
| a stranger's words | **72 %** | **84 %** | 92 % | 73.6 % | 0.791 | 0.676 | **0.405** | 0.885 |

A floor of 0.45 — the knee on the first set — already leaves two descriptions with no result on the second, and 0.50 leaves nine. So the
recommendation in `needs-decision/default-floor.md` is **0.35**, not 0.45 (0.40 survives both, by 0.005). The decision file, the guide, the
README, the CHANGELOG, `rank.rs`, `weights.rs` and `ensemble.rs` were all rewritten to say so. **Had the second wording not been asked
for, the friendly figures would have shipped as the headline.**

**The tool held wrong** (`evidence/after/goldenbench-edge-cases.txt`): no argument; a directory that does not exist; a library with
**1 997 labelled files absent and one stray** (refused by count and by name, "nothing was run"); a queries file with a missing rationale
(*"queries, line 2: `a dog barking` has no rationale"*); no model files (both named); an unknown flag; and `--bless` of another queries
file. Each exits **2**. The last one found a defect: it was refused only *after the whole report* had run, because the check sat where the
baseline is written. It now refuses before anything is loaded.

**Can a developer read the output without the code?** The report names its protocol (*"the paper's protocol: each clip against 'This is
a sound of <class>.', top-1"*), gives the paper's figure beside its own, defines every column of the floor table, shows which sound each
no-answer query found instead, and says in section 6 what it does not do. The one thing it cannot show is whose wording asked the questions:
that is now the first thing the guide says.

**Not covered, said plainly:** any library but ESC-50 (the central limit — see ADR 0009); a GPU; a person hearing anything; the fetch and the
parity script on a Python other than 3.9 with transformers 4.57.6; which of two resamplers is nearer the model's training pipeline; the
Cauldron's panel test (no disk for its 14 GB target). **No test runs the benchmark** — CI has no model and no network.

**Verdict: GREAT** (after three fixes: the early `--bless` refusal, the floor table's footer, and the correction of the headline throughout).
