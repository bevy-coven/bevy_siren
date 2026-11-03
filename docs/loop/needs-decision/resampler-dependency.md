# Should `rubato` join the dependency list, and how does a 44.1 kHz asset reach the model?

## The question

`#12` cannot finish while a 44.1 kHz file is refused, and 44.1 kHz is the commonest rate a game ships. `bevy_siren`
is published at 0.1.0 with a **closed dependency list** enforced by `tests/leaf.rs`, so adding a resampler is a
published promise about the crate, not a line in a manifest.

This file is the decision the loop will not make for you. Everything below it is measured.

## What is already settled, by measurement

| Claim | Evidence |
|---|---|
| `rodio`'s converter **aliases** — linear interpolation up, discarded samples down | its own doc comment, `rodio-0.22.2/src/conversions/sample_rate.rs` |
| a hand-rolled interpolator is worse than nothing here | removed in iteration 62; it produced a plausible embedding from a corrupt signal |
| `rubato 0.16.2` **filters**: 1 kHz keeps **0.9763** of its energy, 15 kHz is intact | `runs/2026-11-03-63/evidence/resampler-probe.txt` |
| it builds in **2.12 s** and holds the crate's MSRV | same |

## The options

### Option A — add `rubato`, behind `semantic`

`semantic = ["dep:ort", "dep:rubato"]`, and `model_buffer` converts instead of refusing.

- **Cost:** a fifth entry in `tests/leaf.rs` and a second new dependency for the feature. `rubato` is pure Rust, so it
  does not add a C toolchain or a download, unlike `ort`.
- **Benefit:** the commonest asset rate stops being an error. A game's existing library indexes as it is.
- **Risk:** the final partial chunk must be **flushed**, or up to `input_frames_next()` samples are lost from every
  file — measured as a 480-sample gap in the probe. A test must pin it.
- **What it costs a consumer who does not enable `semantic`:** nothing. `cargo tree` with no features stays exactly as
  it is, proven the way `ort` was.

### Option B — leave it refused, and require 48 kHz libraries

No new dependency; `BufferProblem::ResamplingRequired` stays, and the documentation says an asset library must be
48 kHz to index.

- **Cost:** a developer with a 44.1 kHz library must convert it outside the tool. That is a real workflow break: Unity,
  Unreal and most asset stores ship 44.1 kHz.
- **Benefit:** the crate's dependency list stays at five.
- **Risk:** the feature looks broken to the majority of users, and a refusal that fires on the common case is the kind
  of loudness that trains people to ignore the crate's messages.

### Option C — resample in the tool server, not the crate

The `[[bin]]` from ADR 0006 does the conversion; the library keeps refusing.

- **Cost:** two code paths — a panel calling the library directly would still be refused while the tool worked.
- **Benefit:** the library's list stays closed.
- **Risk:** ADR 0006's whole point is that the two front ends share **one** library. This would make them disagree
  about which files are indexable, which is exactly the divergence that ADR was written to prevent.

## Recommendation

**Option A.** The measurement is not close: `rubato` filters, `rodio` drops samples, and a hand-rolled one is the
failure this repository keeps paying for. Option B breaks the commonest workflow, and Option C contradicts ADR 0006.

**What lands with it, in the same commit:**

1. `rubato` in `ALLOWED_DEPS` with the sentence that argues for it, exactly as `ort` has.
2. A **flush** for the final partial chunk, and a test that a 1.000 s 44.1 kHz file becomes **48000** samples at
   48 kHz — not 47520.
3. A test that a 1 kHz tone keeps its frequency through the conversion, and that its energy is within the probe's
   measured **0.9763** band.
4. The default build proven unchanged, the way `ort` was: `cargo tree --edges normal` identical with and without the
   feature.

## What this costs, in one line each

| | |
|---|---|
| a user who never enables `semantic` | nothing — the graph is identical |
| a user who does | one more pure-Rust crate, no C toolchain, no download |
| the crate's promise | a fifth dependency, with its argument in `tests/leaf.rs` |
| the future | `S-04` may still replace the index format; nothing here forecloses it |
