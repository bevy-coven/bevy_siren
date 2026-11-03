# Semantic sound selection

> **Status: behind the `semantic` feature, and verified end to end against the model's own numbers.** The audio tower
> runs, and its embedding now agrees with `transformers`' at **cosine 0.999999995** — measured on `laion/clap-htsat-fused`
> through this crate's own spectrogram and an ONNX export of the audio encoder, with the largest single mel value off by
> **0.0545 dB** over 1001 × 64 values. The gap that iteration 76 closed was two constants: the right-hand reflect pad
> (512 samples, not the distance to the last frame's end) and the FFT's precision (`f64`, as the reference promotes to).
> The spectrogram is exact on a single window too, the tokeniser is exact against the model's, and the indexer is
> complete. What is **not** yet measured is anything downstream of a correct embedding — recall against a labelled set is
> still the design defaults (`V-01`). Read the rest with that in mind.

A game's sound library is thousands of files, most of them never named in a cue table. `bevy_siren`'s semantic layer
lets a developer **describe** the sound they want, and finds the files that fit — and, once one is chosen, the others
that go with it.

```toml
bevy_siren = { version = "0.1", features = ["semantic"] }
```

Enabling it downloads ONNX Runtime at build time. A crate that does not enable it is unchanged: `cargo tree --edges
normal` reads the same **413 lines** with and without the feature, measured.

## The six steps, and what each one is

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

**It is binary, and a third of the size.** 5 000 clips weigh **10.28 MB** rather than the text form's **31.88 MB** —
a measurement ADR 0004 made and a test now asserts. The file starts `BSI2` and `read` decides the format from those
four bytes rather than the name, so an index built before this change still loads. A short clip's absent loudness is a
**presence byte**, never a sentinel number:
`-70` is also a real level, and writing it there is the mistake ADR 0003 measured three implementations making.

### 5. Re-indexing embeds only what changed

A library of a thousand sounds gains one file a week, and re-embedding all of them to add one is a minute of GPU for a
second of work. `incremental::update` takes the index a previous run wrote and embeds **only the adds**:

```rust
use bevy_siren::semantic::incremental;

let (index, report) = incremental::update(root, old_index, embed)?;
if !report.is_noop() {
    indexer::write(root, &index)?;      // the only writer; take your lock around this
}
println!("{}", report.describe());      // "1 added, 0 removed, 100 unchanged"
```

Measured: adding one file to a **100-file** library makes **one** call to the embedder.

**Path is not identity, and the type proves it.** `Index::records` is `hash → vector` and holds no paths, so a rename is
indistinguishable from a file that did not move — it costs nothing *by construction*, not by a rule that could be
applied wrongly. A re-encode **is** a new asset, because the bytes changed; the crate cannot tell "re-encoded" from "a
different take", and the other guess would leave the index describing audio that is gone.

**How fast is it?** `cargo run --release --features semantic --example indexbench -- <dir> [clips]` measures the
pipeline in two stages, because one number cannot name the bound: **decode** (scan, decode, downmix, resample and
**loudness**) and **inference**. On an M4 the decode side runs at **2 400–2 700 clips/s**, so a 5 000-clip library costs
**3 s** of decode. The real audio tower is a different scale — ADR 0002 measured **34.9 ms/clip** on a CPU — which is why
inference, not decoding, is what a large library waits on.

`update` never writes. That is deliberate: `indexer::write` owns the file, so a caller composes the two and puts its own
lock around the write — and a caller that only wants to preview a run writes nothing at all.

### 6. Where each sound came from

A library bought or downloaded carries a licence obligation **per file**. `semantic::provenance` records it beside the
asset, and answers the one question a game has to get right:

```rust
use bevy_siren::semantic::provenance::{Provenance, Sidecar};

let sidecar = Sidecar::read(&root.join("licences.csv"))?;
let terms = sidecar.records.get("step.wav").cloned().unwrap_or_else(Provenance::unknown);

match terms.permits_commercial_use() {
    Some(true)  => { /* a licence this crate recognises as commercial */ }
    Some(false) => { /* recognised as non-commercial: do not ship */ }
    None        => { /* nothing recorded, or an identifier it does not know: a person decides */ }
}
println!("{terms}");   // "CC0-1.0, from `kenney-sounds`", or "unknown, source not recorded"
```

**Three answers, and `None` is the important one.** A `bool` defaulting to `true` turns *"nobody recorded this"* into
*"this is fine"*, which is the direction that gets a studio sued. An asset with no row reads `unknown`, and a row that
wrote an empty licence is **recorded** as empty — both answer `None`, and the two stay distinguishable.

**A folder-level `LICENSE` or `README` is never consulted.** An inherited licence that looks authoritative is worse than
an explicit unknown, so a pack whose README says "all CC0" gets unknown provenance for every file. Only a per-file
record counts.

**A build reads the sidecar for you.** Drop it beside the audio and the join happens during `build`: a sidecar names
files, the index names content hashes, and every path a content appears at is checked. Three things a sidecar can get
wrong are reported rather than swallowed — a record naming a file the folder does not have (*"1 licence records matched
no file"*), a sidecar that exists and will not parse (*"Licence record unreadable: …"*), and, importantly, **nothing at
all** when there is no sidecar, which is normal.

Sidecar columns are matched **by name**, so this reads whichever way a pack ordered it:

```csv
source,file,licence,author
kenney,step.wav,CC0-1.0,
```

Two rows for one file is a **conflict** rather than last-write-wins: a licence that is a guess is worse than one that is
unknown.

**Loudness is stored the same way, and its absence is a state.** `Index.loudness` keeps each asset's integrated
loudness, true peak and block counts. A clip too short for one 400 ms gating block has **no** integrated loudness, and
the file writes `-` rather than a number — measured, `ffmpeg` reports `-70.0 LUFS` there, which is the standard's floor
presented as a level *and also a real level*. `-` is the only spelling that keeps "too short to measure" distinct from
"measured at −70".

Everything else in a loudness record is real even for a 0.2 s click: a true peak needs no gating, so it is always
written, along with the two counts that separate *too short* from *silent*.

**A build produces these numbers; you do not supply them.** `build` measures every file it embeds, from the **same
decode** the embedder uses — a second pass would double the cost on a thousand-file library. A measurement that fails
does not fail the file, because the vector is the index's reason to exist, but it is counted: `Built::unmeasured` is
non-zero and `describe()` says so, so a run that measured nothing cannot look complete.

```rust
let built = build(root, stamp, embed)?;
println!("{}", built.describe());          // "1000 indexed", or "1000 indexed, 3 unmeasured"
```

**Brightness and cleanliness are stored too.** `semantic::spectral` gives each asset a **centroid** (the energy-weighted
mean frequency), a **rolloff** (the frequency below which `ROLLOFF_FRACTION` = 0.95 of the energy lies) and a **noise
floor** (the quietest tenth of frames, against the clip's own loudest — a design default). Two recordings of the same
sound in the same room sit close in these; one made in a treated room does not, which an embedding alone may not show.

The centroid is an `Option` for the same reason loudness is: **silence has no brightness**. `0 Hz` is a real reading —
a DC offset measures 9.375 Hz — so returning zero would make silence indistinguishable from a constant, in memory and in
the file.

**Tempo and key are produced by a build too**, from the same decode. The three states they can be in are worth knowing,
because they mean different things to a filter:

| asset | tempo record | key record |
|---|---|---|
| a 4 s click track | `bpm: Some(120.0)` | present, *no key* |
| a 0.1 s clip | present, `bpm: None` | **absent** — shorter than a chroma window |
| never measured | absent | absent |

`None` *inside* a record means **judged and found to have none**; no record means **never measured**. The filter reads the
first as unjudged and the second as unknown.

**It is stored, and it travels with the answer.** `Index.provenance` is a trailing section inside the checksummed body,
so a hand-edited licence is a corrupt file rather than a silent change, and an index with no provenance writes no
section at all — every file written before this existed still loads. `Match` carries `provenance` and is **no longer
`Copy`**, which is the trade for making "forgot to ask" a compile error instead of an unlicensed asset presented as
unencumbered:

```rust
for hit in answer.matches {           // was `Copy`
    println!("{}  {}", hit.provenance, hit.score);
}
```

## Tempo, and the accuracy floor

`semantic::tempo::estimate` returns `Tempo { bpm: Option<f32>, confidence, octave_ambiguous }`. **`bpm` is an `Option`**
because most estimators invent a tempo for a single impact — a steady tone and a lone impact both report *no tempo*, and
the filter treats that as unjudged rather than as a mismatch.

The accuracy floor is ADR 0003's **±2 BPM at 80–160 BPM**, and it is a **test**: the corpus is every whole BPM from 80
to 159, and every one lands inside.

The search range is applied to the **lags**, so an octave error cannot be created by the search, and
`octave_ambiguous` **flags** a strong subdivision rather than "correcting" it — silently halving a tempo is the same
mistake in the other direction.

## Key, and what the measurement changed

`semantic::key` estimates a key by **Krumhansl-Schmuckler**: audio becomes a twelve-class chroma, and each of 24 rotations
of the Krumhansl-Kessler major and minor profiles is correlated against it. On a corpus of **24 keys** it is **100 %
exact** — on synthetic tonal material, which is the easy case and what no asset library resembles.

ADR 0003 had excluded key on the assumption of a *"wrong 40 % of the time"* detector. That figure was never measured;
`needs-decision/key-detection.md` records the measurement and the recommendation that follows.

**An atonal asset reports `Key::absent()`** — a single impact, white noise and silence all do. The compatibility table is a
named serde type, and an **unknown key multiplies by 1.0**, so it never penalises:

```rust
let table = Compatibility::default();
let multiplier = table.multiplier(a.relation(&b));  // 1.0, 0.85 (parallel), or 0.5 (clash)
```

## Filtering, without removing what it could not judge

`semantic::filter` turns measurements into a **policy**: a loudness gap beyond `Weights::loudness_tolerance_db` (6 dB)
removes a candidate, and a noise floor more than `noise_floor_tolerance_db` (12 dB) above the anchor's only *penalises*
it — a hissier take is a worse neighbour, not an impossible one.

**An unknown feature never excludes.** A 0.2 s footstep has no integrated loudness, so the rule cannot judge it, and
removing it would silently delete the commonest asset in a library:

```rust
match filter::evaluate(&anchor, &candidate, &weights).excluded {
    Some(reason) => println!("removed: {reason}"),
    None         => { /* admitted, or admitted unjudged — both are kept */ }
}
```

`filter::filter` returns the kept indices and a `Report` whose `describe()` names the rule that removed the most, so a
set where everything was filtered out reads as a sentence rather than as nothing.

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
