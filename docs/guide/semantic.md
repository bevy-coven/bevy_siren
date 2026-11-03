# Semantic sound selection

> **Status: behind the `semantic` feature, and not yet end-to-end.** Everything below is built and tested except one
> link: **the audio tower has never been run against the real model in this repository**. The spectrogram is exact
> against CLAP's own extractor (measured), the indexer is complete, and the text path is exact against the model's
> tokeniser (measured). What is missing is a verified run of `Model::embed_audio` on a real 116 MB graph, which needs
> the GPU host. Read the rest with that in mind.

A game's sound library is thousands of files, most of them never named in a cue table. `bevy_siren`'s semantic layer
lets a developer **describe** the sound they want, and finds the files that fit — and, once one is chosen, the others
that go with it.

```toml
bevy_siren = { version = "0.1", features = ["semantic"] }
```

Enabling it downloads ONNX Runtime at build time. A crate that does not enable it is unchanged: `cargo tree --edges
normal` reads the same **413 lines** with and without the feature, measured.

## The four steps, and what each one is

```text
  a folder of audio            a description
         │                          │
   ┌─────▼──────┐            ┌──────▼──────┐
   │ library::   │            │ tokenizer   │   CLAP's own vocabulary,
   │ scan        │            │             │   50 265 entries (measured)
   └─────┬──────┘            └──────┬──────┘
   ┌─────▼──────┐            ┌──────▼──────┐
   │ decode +    │            │ Model::      │   512-d, already L2-normalised
   │ resample    │            │ embed_text   │   by the graph (measured)
   └─────┬──────┘            └──────┬──────┘
   ┌─────▼──────┐                   │
   │ mel + embed │                   │
   └─────┬──────┘                   │
   ┌─────▼──────────────┐    ┌──────▼──────┐
   │ Index (content hash│    │ rank::top_k │
   │  stamped by model) │────│ + Floor     │
   └────────────────────┘    └──────┬──────┘
                                    │
                             ┌──────▼──────┐
                             │ Embedder::  │
                             │ ask         │
                             └─────────────┘
```

### 1. Scan: identity is the content hash

`library::scan(root)` walks a directory and returns one `Asset` per **distinct content**, not per path. A renamed file
is the same asset; a re-encoded one is not. Both are measured:

| operation | indexed assets | added | removed |
|---|---|---|---|
| rename `kick.wav` → `kick_renamed.wav` | 1 | **0** | **0** |
| re-encode in place | 1 | **1** | **1** |

A symlink out of the root is refused rather than followed, and every file that is not indexed is reported with a
reason — a corrupt header, an unreadable file, or an extension this crate does not decode are three different answers.

### 2. Decode and resample

Decoding goes through `rodio`, which the crate already depends on for its mixer — so **an asset that plays is the same
asset that indexes**. The rate is read from the decoded stream, not from the header.

Resampling is `rubato`'s FFT resampler, chosen by measurement: `rodio`'s own converter *"discards samples for
down-sampling"* and a hand-rolled interpolator produces a plausible embedding from a corrupted signal. The numbers:

| | |
|---|---|
| 1.000 s at 44.1 kHz | **exactly 48000** samples at 48 kHz |
| a 1 kHz tone | keeps **0.5000 of 0.5000** of its energy |
| 15 kHz | survives; nothing folds to 9.1 kHz |
| the resampler's own latency | **560** samples, skipped |

### 3. The spectrogram is CLAP's, exactly

`mel::log_mel` is the log-mel spectrogram `ClapFeatureExtractor` produces, in this crate with no dependency. Every
constant is read from the model: 48 000 Hz, a 1024-sample Hann window, hop 480, 64 mel bands, 50–14 000 Hz, a
`1e-10` floor.

**It is the same spectrogram, not one that looks right.** A 1 kHz tone at 48 kHz, first frame:

| band | this crate | `ClapFeatureExtractor` | Δ |
|---|---|---|---|
| 0 | 16.570843 | 16.570843 | **0** |
| 1 | 16.544992 | 16.544992 | **0** |
| 2 | 16.996017 | 16.996021 | 4 × 10⁻⁶ |
| 3 | 17.427809 | 17.427816 | 7 × 10⁻⁶ |

The comparison is a test, and its filterbank is the model's own 32 832 weights checked into `tests/fixtures/`.

### 4. The index, stamped by the model it came from

`Index::load(text, &loaded)` takes the running model's stamp as a **required argument**. Vectors from two models compare
without complaint and rank noise, so a mismatch is refused with both versions and the word *re-index*.

The file carries a record count and a checksum, so an interrupted write is detected rather than read as partial data.

## Ask: one call from a description to a ranked answer

```rust
use bevy_siren::semantic::{ask::Embedder, index::ModelStamp, rank::Floor};

let vocabulary = /* Vocabulary::from_json(model's tokenizer.json)? */;
let mut embedder = Embedder::new(&vocabulary, &mut model);
let answer = embedder.ask(&index, &ModelStamp::measured_clap(), "tense, sparse, underwater ruins", 3, Floor::default())?;
println!("{}", answer.describe());   // "3 results"
```

The order inside `ask` is the design: **validate, stamp, encode, embed, rank**. Four other orders are wrong, and only the
first is cheap to get right — a description with no words in it must not reach a GPU.

Every refusal is a sentence naming what to fix:

- *"the description is empty"*
- *"this index was built by a different model than the one loaded. Re-index the library with this model."*
- *"the description needs 128 tokens and the model accepts 77. Shorten it, or split it into two searches."*

## The floor: why the default keeps everything

`Floor` drops results below a cosine. Its default is **0.0**, and that is a measurement rather than timidity.

The correct and incorrect bands **overlap**: a right answer scored **0.126 – 0.429** and a wrong one **−0.104 – +0.150**
in this crate's own end-to-end run. So no single threshold separates them:

| floor | recall@1 |
|---|---|
| **0.000** | **100 %** |
| 0.126 | **100 %** |
| 0.150 | **75 %** |
| **0.500** | **0 %** |

A floor of 0.5 — the number intuition reaches for — rejects **every** correct match. A long list a caller can weigh
beats a short list that has silently dropped the answer. This is a test, not a note:
`a_floor_inside_the_overlap_deletes_right_answers_measured_on_the_real_model`.

## The ensemble: a score that explains itself

`model::Breakdown` keeps **four facts per signal** — the raw value, the weight, the product, and whether the signal had a
value at all. The last one matters: a tempo never estimated and a tempo that disagreed both contribute `0.0`, and a raw
cosine in `[-1, 1]` looks identical to a loudness penalty in decibels once multiplied.

```text
semantic carried it (+0.400 from 0.4 raw at weight 1);
acoustic counted against it (-0.100 from -0.2 raw);
no value for tempo, key
```

That is the whole of an `explain_match` in a string. The arithmetic lives in `src/model.rs`, which no engine type may
appear in — `tests/all.rs` enforces it, and a second test asserts the ensemble is inside that scan.

## Tuning, as data

`semantic::weights::Weights` holds the six numbers the ensemble uses, each a **design default** with the argument for
its value in its own doc comment. Two rest on this crate's measurements: `acoustic` at 0.5 because the bands overlap, and
`drop_fraction` at 0.0 because a filter that hides the right answer is worse than a long list.

`Weights::problems()` reports what cannot mean anything — all weights zero, a negative weight, a `NaN`, a zero
tolerance — while **allowing what is merely unusual**, such as a 40 dB tolerance for a stylised palette whose loudness
jumps are intended. A report that refuses a legitimate choice is a report people learn to ignore.

## What this layer does not do

- **It does not open a device.** Everything here is arithmetic and files; playback stays the plugin's job, and an
  audition goes through the same `Play` message as any other cue.
- **It does not download weights.** `Model::load` takes a path that already exists.
- **It does not decide what a model may be.** Which checkpoints a project may ship is a licence question, recorded in
  `docs/loop/needs-decision/`, not a fact this crate asserts.
- **It has not run the audio tower against the real model.** The spectrogram is exact, the indexer is complete, and
  `Model::embed_audio` is written — but the one link that needs a GPU host is unverified, and this page says so rather
  than implying otherwise.
