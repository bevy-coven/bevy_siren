# Iteration 78 — eval

A **docs-only** iteration: `docs/adr/0005-models-and-licences.md` plus its run evidence. `git diff --name-only HEAD -- src/ tests/ Cargo.toml` returns **0 files**.

## The cases

Because no compiled code changed, the audio cases (1–33) are inherited from iteration 77 exactly, not re-asserted:
same sources, same tests.

| Check | Result |
|---|---|
| files changed under `src/`, `tests/`, `Cargo.toml` | **0** |
| `cargo fmt --check` | **pass** |
| `cargo test --lib --features semantic` | **BLOCKED — disk** (`libbevy_pbr … No space left on device`) |
| every other baseline step | **BLOCKED — disk** |

## Baseline

The volume sat at **122–152 MB free** for the whole iteration; `cargo clean -p` on the heavy deps bought 1.2 GB, which
`bevy_pbr`'s archive consumed in one link. `du` over the home directory times out at 120 s at this occupancy, so even
finding the consumer is not possible without a decision about what may be deleted.

This is the **fourth** consecutive iteration the disk has cost, and the first where the chosen item was selected
*because* it needed no build. That is a workaround, not a fix.

## The audit, which is this iteration's measurement

Read from the Hugging Face API on 2026-11-03 and recorded verbatim in `evidence/model-licences.txt`:

| Model | Licence | Commercial use |
|---|---|---|
| `laion/clap-htsat-fused`, `-unfused` | **Apache-2.0** | **yes** |
| `m-a-p/MERT-v1-95M`, `-v1-330M` | **CC BY-NC 4.0** | **no** |
| `OpenMuQ/MuQ-large-msd-iter`, `MuQ-MuLan-large` | **CC BY-NC 4.0** | **no** |
| `minzwon/musicfm-v-demo`, `nvidia/audio-flamingo-hf`, `EffOrg/EF-CLAP` | **no licence tag** | **no grant** |
| LAION-audio-630k corpus | **no licence file** (404; GitHub `license: null`) | **open risk** |

**Every music-specific encoder on the shortlist is non-commercial**, which retires #39's premise. And #6's own
insistence was right: the corpus has no licence at all and includes purchased commercial libraries, so the Apache-2.0
weights sit on an unclear training grant. That is recorded as an open risk with a named escape hatch — consume an index
as data via `ModelStamp` — rather than resolved by assertion.
