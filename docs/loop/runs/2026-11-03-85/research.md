# Iteration 85 — research: measuring during the build, and the block count that was wrong in three places

The item is the wiring named at the top of iteration 84's list: `loudness::measure` and `Sidecar::read` both existed and
were tested, and **neither was called by `indexer::build`**. This iteration calls the first.

## Sources

| Source | The rule it reports |
|---|---|
| `src/semantic/indexer.rs::embed_one` | decodes, downmixes, resamples to `MODEL_SAMPLE_RATE`, and hands the buffer to the embedder — **the exact buffer `loudness::measure` needs** |
| `src/semantic/loudness.rs::measure` | takes `(&[f32], rate)`, so it can run on that same buffer with no second decode |
| `ebur128` / BS.1770 (via ADR 0003) | 400 ms gating blocks hopping **100 ms** — 75 % overlap |
| `src/semantic/loudness.rs`, this iteration's arithmetic | `1 + (len − block) / hop`, recomputed and checked: **1 s → 7 blocks, 3 s → 27** |
| `runs/2026-11-03-77` | the −23.0 LUFS measurement a 0.1-amplitude tone gives, which the new test uses as its anchor |

## The technique, in the sources' terms

An index build already touches every file's decoded samples. Measuring loudness needs *the same samples*, so the two
must share one decode or the feature doubles the cost of the operation it exists to make cheap on a thousand-file
library. `embed_measured` therefore does decode → downmix → resample **once** and hands the buffer to `measure` and then
to `embed`.

The policy question is what a **failed measurement** means. It is not a failed file: the vector is the index's reason to
exist, and a loudness number is an addition to it. But dropping it silently would make a run that measured nothing look
complete — the silence this crate refuses everywhere — so `Built` carries `unmeasured: usize` and `describe()` prints it.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| loudness input | mono at `MODEL_SAMPLE_RATE` | `embed_one`'s existing output |
| decodes per file | **1** | shared between `measure` and `embed` |
| gating block / hop | 400 ms / 100 ms | BS.1770 via ADR 0003 |
| blocks for 1 s | **7** | `1 + (48000 − 19200)/4800` |
| blocks for 3 s | **27** | `1 + (144000 − 19200)/4800` |
| a 0.5-amplitude tone | ≈ **−9.0 LUFS** | `0.1` gives −23.0; `20·log10(5) = 13.98 dB` louder |

## What good sounds like — and the defect this found

A build of a three-second tone stores a loudness of about **−9 LUFS with 27 blocks**, and a build of a 0.2 s click stores
`None` with a real peak and **0** blocks possible. A run where measurement produced nothing says *"3 unmeasured"* rather
than *"3 indexed"*.

**The number 27 is the finding.** The documentation in `loudness.rs` claimed a 3 s clip has **7** gating blocks, the
existing test's message repeated it, and my new test asserted it — because 7 is the *non-overlapping* count
(`3.0 / 0.4`). A 75 %-overlapping block advances 100 ms, so a 3 s clip has 27. The existing assertion was
`blocks >= 7`, which **passes on 27** and therefore hid a false statement in shipped doc text. Only a new test that
asserted the exact number exposed it.
