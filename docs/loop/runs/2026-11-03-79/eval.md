# Iteration 79 — eval

A **docs-only** iteration: `docs/adr/0004-vector-store.md` plus run evidence. `git diff --name-only HEAD -- src/ tests/
Cargo.toml` returns **0 files**.

## The cases

No compiled code changed, so cases 1–33 are inherited from iteration 78 exactly — same sources, same tests.

| Check | Result |
|---|---|
| `git diff --name-only HEAD -- src/ tests/ Cargo.toml` | **0 files** |
| `cargo fmt --check` | not required (no Rust changed) |
| every cargo step | **BLOCKED — disk**, `148–449 MB` free; one `write` of a 7 KB file failed `ENOSPC` outright |

## Baseline

The volume had **148 MB** free at the start; `cargo clean -p bevy_pbr` bought 326 MB, which was enough to write files
and not enough to link. **Fifth consecutive iteration the disk has cost.** The consumer is identified this time and
does not belong to this repo:

| Path | Size |
|---|---|
| `~/.cache/huggingface/hub/models--black-forest-labs--FLUX.2-klein-9B` | **33 GB** |
| `~/.cache/huggingface/hub/models--black-forest-labs--FLUX.2-klein-4B` | **15 GB** |
| `~/bevy_siren/target` | 16 GB |
| `~/.cargo` | 8.7 GB |

`~/.cache` alone is **50 GB**. Deleting either FLUX checkpoint recovers 15–33 GB instantly, and neither is a Rust build
artifact — they are machine-learning weights belonging to the user's image tooling, so they are **not** ours to remove
without a decision.

## The measurement, which is this iteration's evidence

Sizes computed from the crate's own `to_text` format (mean **6 376.8 bytes** per record over 2 000 synthetic 512-d
vectors), and latencies measured in memory on this machine:

| Clips | Text (today) | Binary f32 | Binary f16 |
|---|---|---|---|
| 500 | 3.19 MB | 1.03 MB | 0.52 MB |
| **5 000** | **31.88 MB** | **10.30 MB** | **5.20 MB** |
| 50 000 | 318.84 MB | 102.99 MB | 52.01 MB |

| Query | Time |
|---|---|
| BLAS f32 dot, 5 000 × 512 | **0.05 – 1.87 ms** |
| Scalar f32 dot (one core) | ~2–4 ms |

**The issue's premise is corrected**: its "5 MB index or a 40 MB SQLite file" compares against a size the crate is
already at — the existing text format is 31.9 MB at 5 000 clips, and 5 MB requires `f16`.
