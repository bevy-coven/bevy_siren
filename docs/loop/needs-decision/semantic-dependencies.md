# Which dependencies may the semantic layer add, and does `tests/leaf.rs` allow them?

## The question

`tests/leaf.rs` closes this crate's dependency list at **`bevy`, `serde`, `ron`, `rodio`**, and that test is what keeps
`bevy_siren` a sound crate rather than a framework. Semantic selection needs at least three more kinds of thing:
**a decoder** (or a feature of the one it already has), **an ONNX runtime**, and **a way to persist an index**.

Issue [#10](https://github.com/bevy-coven/bevy_siren/issues/10) exists to extend that list deliberately, before any
dependency lands. This file is the decision it needs, and it is a decision rather than an implementation detail because
**each entry changes what a consumer's build costs** and because the list is a published promise.

The measured facts that narrow it:

- [#2's spike](../../runs/2026-11-03-62/research.md) proved the CLAP audio tower exports exactly (cosine 1.000000000,
  39 standard operators), so an ONNX runtime is viable and a Python sidecar is not needed.
- [#9](https://github.com/bevy-coven/bevy_siren/issues/9) proved the `semantic` feature adds **no** dependency today:
  `cargo tree --edges normal` is 413 lines either way. So the list is still closed, and the question is what opens it.
- `rodio` is **already** a dependency, behind `mixer`, with `wav` and `vorbis` enabled. It can decode. It cannot
  resample.

## The options

### Option A — the narrowest opening: `ort` only, and reuse `rodio` for decoding

`semantic = ["dep:ort", "rodio"]`, and decoding goes through the `rodio` the crate already depends on.

- **Cost:** `ort` downloads an ONNX Runtime binary at build time; that is a real platform burden on a crate whose
  current build is pure Rust plus one audio library. The semantic feature implies `mixer`'s rodio, so a user who wants
  retrieval also links the mixer's decoder — a coupling that is defensible (both need to decode) but must be stated.
- **Benefit:** **one** new entry, and the decoder is one this crate already trusts and already tests.
- **Risk:** resampling. `rodio` decodes to the file's own rate; CLAP wants 48 kHz. Nothing in the current graph
  resamples. If no `rodio` source does it, this option is incomplete.

### Option B — `ort` plus an explicit resampler crate

`semantic = ["dep:ort", "dep:rubato", "rodio"]`, with `rubato` doing sample-rate conversion.

- **Cost:** two new entries. `rubato` is a real dependency with its own MSRV to hold at or below 1.97.1.
- **Benefit:** resampling is a solved problem with a measured quality, rather than a hand-rolled linear interpolation
  that would quietly colour every embedding.
- **Risk:** `rubato` is designed for streaming with buffers; an offline whole-clip conversion is a different use of its
  API and must be measured, not assumed.

### Option C — a hand-written resampler in `src/model.rs`'s spirit

No new dependency: a windowed-sinc conversion written in the crate, tested against a tone.

- **Cost:** the crate owns a DSP correctness problem, and a resampling bug changes every embedding without failing
  anything.
- **Benefit:** the dependency list stays at five entries, which is a strong promise for a sound crate.
- **Risk:** this is the option most likely to be subtly wrong. A 44.1→48 kHz conversion with a bad filter aliases, and
  an aliased mel spectrogram is still a plausible-looking vector.

### Option D — the sidecar, revisited

A Python process does decode, resample and embed; Rust only speaks to it.

- **Cost:** a user needs Python and torch. This contradicts the "costs little to run" goal and reverses ADR 0002.
- **Benefit:** no new Rust dependencies at all, and the boundary is a pipe.
- **Risk:** ADR 0002 rejected this on measurement, not preference, and nothing has changed there.

## Measured: `rodio` cannot resample correctly

Read from the installed source at `rodio-0.22.2/src/conversions/sample_rate.rs`, on `SampleRateConverter`:

> *"The converter uses **simple linear interpolation for up-sampling and discards samples for down-sampling**. This may
> introduce audible distortions in some cases (see #584)."*

Its own doc comment also names a limitation for rates that are mutual primes.

**Discarding samples for down-sampling is aliasing**, not resampling. For this feature the consequence is specific and
bad: 44.1 kHz in, 48 kHz out means the mel spectrogram is built from an aliased signal, the embedding is a plausible
vector, and **nothing fails** — which is exactly the failure mode Option C was rejected for.

So **Option A is out**, measured rather than assumed. `rodio` decodes, and does not resample well enough to feed a
model.

## Recommendation

**Option B: `ort` plus an explicit resampler.** `rodio` is ruled out by its own documentation (above), and Option C is
ruled out for the same reason that made rodio unsuitable: this repository keeps paying for defects that produce a
plausible number instead of a failure, and an aliased embedding is precisely that.

**What must be recorded before either lands:**

1. ~~Whether `rodio 0.22` can convert a 44.1 kHz source to 48 kHz.~~ **Done: it cannot, correctly.** See the section
   above; the source says linear interpolation up and discarded samples down.
2. `ort`'s and any resampler's MSRV against this crate's floor of **1.97.1**, which is Bevy 0.20's floor rather than one
   above it.
3. The platform story: `ort` must build on macOS arm64 and Linux x86_64+CUDA, and the semantic feature must build on a
   runner with **no GPU** — that is the CI step [#9](https://github.com/bevy-coven/bevy_siren/issues/9) added.

## What each option costs a consumer

| | new deps | build burden | what can go silently wrong |
|---|---|---|---|
| A | 1 | ONNX Runtime download, mixer's rodio | resampling, if rodio cannot do it |
| B | 2 | ONNX Runtime download, mixer's rodio, rubato | a resampler API used wrongly offline |
| C | 0 | nothing | an aliasing bug in every vector |
| D | 0 Rust | Python and torch on the user's machine | everything behind a pipe |

## Why this is not decided here

The repository's rule is that a public API or dependency change goes to `needs-decision/` and the run continues with
the rest of the item. The rest of #12 — reading a file's rate and channels, and producing the buffer shape the model
wants — does not depend on which option wins, so that part proceeds.
