# Iteration 77 — plan

## The item

[#4](https://github.com/bevy-coven/bevy_siren/issues/4): *"ADR: acoustic feature extraction — which crates, and what
lives in the sidecar."* Priority **p0**, labelled `epic:foundations`, and **nothing blocks it**. It in turn blocks
**five** other issues (#21, #22, #23, #24, #25), which block #27, which blocks #40.

## Ranked list

| # | Item | Size / priority | Why it ranks here |
|---|---|---|---|
| **1** | **#4 the acoustic-features ADR** | S–M / **p0** | Unblocks five issues at once and needs no build. It is the only p0 item with nothing above it. |
| 2 | #21 integrated loudness and true peak | M / **p0** | The most valuable *implementation*, and it is blocked on #4 — so #4 is strictly first. |
| 3 | #14 per-asset licence and source metadata | S / **p0** | Independent of #4, and cheap. |
| 4 | #16 incremental indexing by content hash | M / **p0** | The `fnv1a` in `library.rs` is already the seed for it. |
| 5 | #5, #6 the vector-store and licence ADRs | M / **p0** | Also unblocked, but neither unblocks a chain the way #4 does. |
| 6 | #28 the MCP tool host | M / p1 | Iteration 76's first item, but it consumes an index whose feature layer is unresolved. |

**Why (1) beats (2) in one sentence:** #21 cannot start until #4 says which crate measures loudness, and #4 is a
document that the full disk cannot block — so it is both the larger unblock and the cheaper one.

## What "done" means for an ADR

The issue's acceptance criteria, each met by a specific section:

| Criterion | Where |
|---|---|
| an ADR exists at `docs/adr/0003-acoustic-features.md` | the file |
| each of the six features has a named crate or an explicit sidecar assignment | the decision table, six rows |
| loudness validated against a reference on the same file, delta recorded | the measured table in the ADR and `research.md` |
| tempo and key each state a measurable accuracy floor or are declared out of scope | the tempo section (**±2 BPM**, design default) and the key section (**out of scope**) |

## The design, and the one dependency it costs

**Decision: all six features are Rust; no sidecar.** The load-bearing fact is that **`ebur128` 0.1.11 is pure Rust** —
its `src/` is nine `.rs` files, its C files are confined to `capi-test/` and `tests/c/`, and its `build.rs` only runs
under the `c-tests` feature, which is **off by default**. The C in the package exists to be the conformance oracle.

That costs exactly one new entry in `ALLOWED_DEPS` (`tests/leaf.rs`), added in the same commit with the sentence that
argues for it, which is what that ratchet is for.

| Feature | Landed where | Citation |
|---|---|---|
| integrated loudness | `ebur128`, `default-features = false` | ITU-R BS.1770-4 |
| true peak | `ebur128`, `Mode::TRUE_PEAK` | BS.1770-4 Annex 2 |
| spectral centroid | hand-rolled over the existing FFT | **design default** (definition, not threshold) |
| spectral rolloff | hand-rolled, same FFT | **design default** (definition, not threshold) |
| noise floor | hand-rolled, same FFT | **design default** |
| tempo | hand-rolled autocorrelation, **±2 BPM floor** | **design default**, argued in the ADR |
| key | **out of scope** for the Rust path | stated because no Rust implementation reports a usable figure |

## Modules touched

This iteration touches **no `src/`**: the deliverable is a decision document, and the ADR's own "what this costs"
section names the modules that will change (#21 adds a loudness analyser; #22–#25 add feature extractors). The
dependency list is unchanged **in this commit** — extending `ALLOWED_DEPS` belongs to the commit that first declares
`ebur128`, not to the ADR that recommends it.

## Tests that prove it

An ADR is proved by the **measurement** it records, not by a test of its own. The numbers here come from
`ffmpeg`'s `ebur128` on three files, run in this iteration and reproduced in `research.md`. What a later issue must
test is #21's, and this ADR fixes the tolerance: **0.1 LU** for integrated loudness, **0.1 dB** for true peak, plus the
short-clip decision as a named case.

## Carved off, deliberately

- **Implementing the loudness analyser.** That is #21, and it needs `ebur128` in the manifest, which is a dependency
  change that must be its own commit with its own test.
- **Writing the tempo estimator.** #24, and its floor is the risk this ADR accepts.
- **The sidecar design.** The ADR names the exact triggers that would promote one; building it before a trigger fires is
  the cost this document exists to avoid.
