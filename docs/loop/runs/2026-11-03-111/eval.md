# Iteration 111 — eval

## Baseline before anything (2026-11-03 19:55, commit `5262f4f`, a clean tree)

**Red on exactly one thing: the disk.** `df -h ~` reads **4.6 GB free (100 %)**. `target/` is 26 GB (`debug` 22 GB: `deps` 13 GB,
`examples` 8.7 GB; `release` 4.4 GB). A rebuild of any feature set links another ~1 GB test binary, so a semantic change here
cannot be built without clearing space first (the skill ranks this above every feature).

Everything that could run on the cached artifacts was green:

| run | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 |
| `cargo test --features mixer` | 209 + 6 |
| `cargo test --features semantic` | 109 lib + 482 + 6 |
| `cargo test --all-features` | 109 lib + 506 + 6 |
| `cargo test --doc`; `--features readme-example --test all readme::` | 6; 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |
| `cargo build --examples` (default, and `--features semantic`) | ok |

The 33 cases of `cues.yaml` are asserted by the matrix above (and the 21 rendered WAVs are compared again at the end); none is red.
No half-finished work: a clean tree, and every `runs/` directory has its `review.md`.

## The disk

`cargo clean -p bevy_siren --target-dir target --dry-run`: **834 files, 14.8 GiB, this crate's own artifacts only** (a dependency is
never named). Provenance: the oldest of them is today, 16:46, and the newest 19:54 — every one is a product of `cargo build/test/run`
that I ran in this session, which the rules allow me to delete (and the skill names this exact command for this exact condition).
`~/.cache/huggingface` is the owner's and is not touched.

## After

| run | before | after |
|---|---|---|
| `cargo test` | 199 + 6 doc | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 | 208 + 6 |
| `cargo test --features mixer` | 209 + 6 | 209 + 6 |
| `cargo test --features semantic` | 109 lib + 482 + 6 | 109 lib + **497** + 6 |
| `cargo test --all-features` | 109 lib + 506 + 6 | 109 lib + **521** + 6 |
| `cargo test --doc`; `--features readme-example --test all readme::` | 6; 1 | 6; 1 |
| clippy `--all-targets --all-features -D warnings`; `cargo fmt --check`; `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean | clean |
| `cargo build --examples` (default, and `--features semantic`) | ok | ok |
| `render`: WAVs against iteration 110's | — | **21 of 21 byte-identical**; master −21.5 LUFS, −7.5 dBFS TP; `mix.png` and `footstep-grass-spectrum.png` read |

The test count moved by **+15**: 19 new tests (`tests/semantic_golden.rs`) and **4 deleted** (`semantic_query.rs` ×3, `semantic_rank.rs` ×1 —
tautologies on literals and a default-pin whose stated reason became false; the CHANGELOG names each). `a_floor_inside_the_overlap_…`
was rewritten as `a_floor_drops_a_right_answer_that_scores_under_it`. The Cauldron's panel test was **not run**: this disk has ~10 GB
free and the skill asks for a 14 GB target directory, and nothing the panel reads changed (`CueTable`, `SirenSettings`, `MusicScore`
and their RON are untouched; every public addition is behind `semantic`).

## The real-model runs (`evidence/after/`)

| proof | file | result |
|---|---|---|
| the fetch repairs a damaged set and refuses a padded one | `fetch-repair.txt` | a stray `.wav` refused (exit 1); a missing and a corrupted file fetched again and verified on arrival; then nothing to do |
| the benchmark on 2 000 real clips | `goldenbench-first-run.txt`, `goldenbench-run.txt` | zero-shot 87.8 %, hit@1/3/10 90/100/100 %, P@10 88.8 %, MRR 0.950, mAP@10 0.857; index 211 s |
| independent reference, 44.1 kHz, all 2 000 clips | `golden-parity.txt` | **zero-shot 87.8 % in both**; 232 000 similarities: mean difference 8 × 10⁻⁴, 99.9th percentile 0.03, max 0.089; the Python and Rust metric code agree to the printed precision |
| the floor analysis | `floor-analysis.txt`, `floor-analysis-paraphrase.txt` | AUC 0.963 (a stranger's wording: 0.885); a margin separates nothing (0.496) |
| **fresh eyes: a stranger's wording** | `goldenbench-paraphrase.txt` | **hit@1/3/10 72 / 84 / 92 %**, P@10 73.6 %, MRR 0.791, mAP@10 0.676 — and the safe floor falls from 0.45 to 0.35 |
| every refusal path | `goldenbench-edge-cases.txt` | no args, no directory, a library with 1 997 files absent and one stray, a queries file with no rationale, `--bless` of another queries file, no model, an unknown flag: each exit **2**, each naming what |

## What went wrong on the way (all mine; each found by running it)

1. **The first download route took two hours.** GitHub's archive streamed at ~100 KB/s; killed at 193 MB after 30 minutes. The script now
   fetches files, 16 at a time, each verified as it arrives: **79 s**.
2. **Two quirks in the dataset's own `LICENSE`** stopped the fetch — `5-141683-Ad` for `5-141683-A`, and a title written `[DrinkingCan[Norberd]](url)`. A
   script that tolerated a missing licence line would have shipped 2 clips with no licence; this one failed loudly and now names both.
3. **A blind line-number edit** to `golden_parity.py` replaced `def unit(v):` instead of the path it meant to fix; the next run died on
   an `IndentationError`. Re-read, repaired, and every later edit used lines from a read or a grep.
4. **rustdoc**: the module map linked `[`decode`]` etc., which resolve from the crate root — so `model` silently linked the *wrong*
   module (`crate::model`) and the rest were errors. Plain names now.
5. **The baseline's identity omitted the queries file**, so a run with different questions would have been compared with the recorded
   numbers and reported a false regression. The identity now includes it, and another queries file is compared with nothing.
6. **`--bless` of another queries file was refused only after the whole report** had run; it is refused before anything loads.
7. **The recommendation changed.** On the maintainer's wording a floor of 0.45 looked like the knee (every top result kept). The
   paraphrase run — written to test exactly that — deleted answers at 0.45, so the recommendation is 0.35. Every figure that said "every
   description finds a right answer" now says in whose words.
