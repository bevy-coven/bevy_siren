# Semantic sound selection

> **Status: behind the `semantic` feature, and verified end to end against the model's own numbers.** The audio tower
> runs, and its embedding now agrees with `transformers`' at **cosine 0.999999995** — measured on `laion/clap-htsat-fused`
> through this crate's own spectrogram and an ONNX export of the audio encoder, with the largest single mel value off by
> **0.0545 dB** over 1001 × 64 values. The gap that iteration 76 closed was two constants: the right-hand reflect pad
> (512 samples, not the distance to the last frame's end) and the FFT's precision (`f64`, as the reference promotes to).
> The spectrogram is exact on a single window too, the tokeniser is exact against the model's, and the indexer is
> complete. **Recall is now measured**, on 2 000 labelled recordings ([the golden set](golden-set.md)): described in the maintainer's
> words, every one of 50 descriptions finds a right answer in its first three; in a stranger's words, 84 % do. The crate reproduces
> `transformers`' zero-shot accuracy on the same audio to four places (87.8 %). What is **not** yet measured is a person's judgement
> of which sounds *fit* an anchor (`#35`'s third criterion), music, speech and game foley (ESC-50 has none), and the floor on any
> library but that one.

A game's sound library is thousands of files, most of them never named in a cue table. `bevy_siren`'s semantic layer
lets a developer **describe** the sound they want, and finds the files that fit — and, once one is chosen, the others
that go with it.

```toml
bevy_siren = { version = "0.1", features = ["semantic"] }
```

Enabling it downloads ONNX Runtime at build time. A crate that does not enable it is unchanged: `cargo tree --edges
normal` reads **413 lines** by default and **493** with `semantic`, 149 crates becoming 185 — the file decoders (`rodio`'s
`symphonia` codecs for WAV, FLAC and MP3 and `lewton` for Ogg Vorbis: 18 crates, counting what they pull in), the runtime, the resampler, the
loudness analyser and `serde_json` — measured in this crate's own graph, which builds `bevy` with its defaults off; a game's
graph already holds the formats its own `bevy` enables, Vorbis (through `lewton`) among them by default, so for Vorbis it adds none.

## Try it: `examples/semantic`

One command on a folder of WAV, Ogg Vorbis, FLAC or MP3 files: build (or reuse) the index, describe a sound, read the best three with
the reason for each, and **hear** the first through the crate's own play path. The crate does not ship the model, so the
first time is two steps:

```sh
# once: both graphs and the tokenizer, from the weights already in your Hugging Face cache (needs torch, transformers,
# onnx and numpy; with laion/clap-htsat-fused cached and HF_HUB_OFFLINE=1, nothing is downloaded)
HF_HUB_OFFLINE=1 python3 tools/export_clap_onnx.py ~/clap

export SIREN_MODEL=~/clap/clap-text.onnx SIREN_VOCAB=~/clap/tokenizer.json SIREN_AUDIO_MODEL=~/clap/clap-audio.onnx \
       SIREN_MODEL_VERSION="$(cat ~/clap/clap-export.txt)"
cargo run --release --features semantic --example semantic -- <a folder of audio> "a short electronic beep"
```

![`examples/semantic` as a terminal transcript: the first run builds the index of fourteen files with the real CLAP audio tower and plays the best match for "a short electronic beep" through the Music bus; the second reads the index back and ranks "a frog croaking", a miss, with the reason under every result.](../media/semantic.png)

Under each result are two lines. The first is **why it ranked**: *"semantic carried it … no value for acoustic, tempo, key"*
means only the description's similarity counted, because an unanchored search has no sound to compare acoustic features,
tempo or key with — name an anchor (`siren-tools`' `anchor` argument) and those terms join in. The second is **where it
came from**: *"unknown, source not recorded"* is what a folder with no licence sidecar says, and the crate says *unknown*
rather than leaving a reader to take silence for permission.

**With nothing set up it says what is missing and stops.** There is no GPU path in this build — the model runs on the CPU
in fp32 and the first line says so — so *no GPU* is not a failure; a missing file is. It lists every missing file
(`--model`, `--vocab`, and `--audio-model` when the folder needs indexing) with how to make them, and exits with status
**2** before it reads the library, writes an index or opens the output device.

**The second run reads the index the first wrote**: *"14 assets, read from `siren-index` — current, so no audio graph was
loaded"*. The example treats that file as a **cache**: a folder that gained or lost a file, or an index it cannot use
(another export's, or a damaged one), is **rebuilt whole and written over**, saying so first — so adding one file to a
5 000-clip library costs the full five to nine minutes here, where `incremental::update` would embed one clip; the library has no
model-backed form of that yet, only the closure it takes. `--model-version` (or `SIREN_MODEL_VERSION`) names the export that
made the vectors — the line `clap-export.txt` holds — so an index from your torch is labelled as yours rather than refused
as the measured one's; `siren-tools` takes the same flag. `--k`, `--pick`, `--bus` and `--no-play` are in the example's own
usage line.

**What the export script checks, and the trap it found.** The text graph is compared with `get_text_features` (largest
difference **0.0**) and the audio graph with the audio tower on the path the crate runs (cosine **1.000000000**). That
path is `is_longer = false`, and it is **not** `get_audio_features`' default: the extractor marks one clip of every fused
batch `is_longer = True` — even a lone one-second clip — which turns on the local-mel fusion branch and moves the
embedding by cosine **0.72–0.83** across the clips tried. The crate sends `false` for every window (it chunks anything
past ten seconds, so none is "longer"), which is LAION-CLAP's own behaviour for a short clip. An export traced with the
extractor's tensor baked the fusion branch in, and the crate's `false` then ran that branch on an empty batch — *"Input
shape {0,96,64,3,21}, requested shape {0,96,64,-1}"* — so the first real run indexed **0 of 14** files. The example now
refuses to write an index with nothing in it.

**It agrees with `transformers`.** `docs/loop/runs/2026-11-03-110/evidence/after/parity.py` puts the same five phrases to
the same 14 files through both and compares every similarity: the largest difference is **4.9 × 10⁻⁴** — the example
prints three decimals, so that is the rounding — and the ranking order is identical for every phrase.

**What it says about quality, honestly.** On fourteen macOS system sounds, "a short electronic beep" finds `Sosumi`
(cosine 0.21) and "a frog croaking" does not find `Frog`: CLAP ranks it eleventh of fourteen, at −0.02, because it is a
synthetic blip and not a frog. A cosine is a position on CLAP's own scale, and 0.2 is a good match
([the floor](#the-floor-what-2-000-real-recordings-say) has the measured bands); the three results are a short list for
a person to choose from, which is what the paper measures — mAP@10 of 24.9 for text-to-audio on Freesound.

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

### 2b. Formats: what each was checked against

`library::AUDIO_EXTENSIONS` is `wav`, `ogg`, `oga`, `flac`, `mp3`, and **a test decodes a real file of every one of them** — for a long time
two were on the list and could not be decoded at all, because nothing had ever opened a file that was not a WAV. Each was then checked
against a decoder that is not this crate's, on real files, through the whole pipeline (`formats_parity.py`: the crate on the file against the
crate on the same file decoded by the reference, so a difference is the decoder's alone):

| format | checked against | result |
|---|---|---|
| WAV | — | the golden set's own format; the baseline |
| FLAC | 2 000 ESC-50 clips as WAV and as FLAC, all 232 000 similarities | **identical** — a largest difference of 0.0 at six decimals |
| MP3 | Kenney's 62 MP3 effects against `ffmpeg`'s decode, 12 descriptions | **identical** to the printed three decimals on all 259 pairs; every length exact |
| Ogg Vorbis | 254 Kenney Ogg effects against `libvorbis` (iteration 115, `lewton`) | samples agree to **7 × 10⁻⁷** at worst, hot masters included; **the stream's end came back up to 21 ms late in 51 of 254 files — cut** (below). On `symphonia`, which this crate used until iteration 115, 209 files agreed to 1.8 × 10⁻⁷ relative rms with the end late in 59 |

**Ogg: the decoder is `lewton`, and the end is cut to the length the stream declares.** `rodio` has two Vorbis decoders and this crate's `Cargo.toml` picks `lewton` — the one Bevy's own `vorbis`
feature selects. `symphonia`'s made **a loop of any Ogg silent** (`repeat_infinite` of it produced no samples: RustAudio/rodio#775) and, because Cargo unifies features, would have swapped the decoder of a game's
*own* audio too; against `libvorbis` on 254 real files it also **clamped** hot masters to [−1, 1], off by up to 0.30 where `lewton` is within 7 × 10⁻⁷ (and, built with `Decoder::new` as the mixer builds it, returned more than the stream declares for 253 of them)
(`docs/loop/runs/2026-11-03-115/evidence/after/vorbis-backend-parity.txt`). The Kenney golden benchmark with the real CLAP model **did not move on any metric** — zero-shot 0.3729, hit@1 0.3103, hit@3 0.5172, hit@10 0.6552,
MRR 0.4418, mAP@10 0.2577, all `+0.0000` against the recorded baseline (`goldenbench-kenney-lewton.txt`).

Neither decoder cuts the padding of the last Vorbis block, which the spec says to discard, for every file: with `lewton` 51 of 254 came back up to 936 frames late (21 ms at 44.1 kHz; the loudest padding at −6 dBFS, the median
at −49), and CLAP repeats a short clip to fill its 10 s input — on the RPG pack's effects that tail alone moved a similarity by up to 0.085 and changed the top three of **all 12** descriptions tried. `decode` cuts an Ogg
that is one stream ending cleanly to its last page's granule position: against `libvorbis` the worst difference is 0.004. A chained or truncated file is left alone. **`mixer` and Bevy's `AudioPlayer` cut a
*loop* the same way** (`crate::ogg` is the one walk of the pages all three use, so what is indexed and what loops cannot disagree), because the padding is what sits between two passes; a one-shot just ends in it.

**A lossy copy of a library is a different library.** The same 2 000 ESC-50 clips, re-encoded and put through the same benchmark:

| | median change in a similarity | zero-shot | hit@1 (maintainer's / stranger's wording) | hit@10 (stranger's) |
|---|---|---|---|---|
| WAV | — | 87.8 % | 90 % / 72 % | 92 % |
| FLAC | **0** | 87.8 % | 90 % / 72 % | 92 % |
| MP3 128 kbit/s | 0.005 (mean 0.008, p99 0.057) | 87.5 % | 94 % / 74 % | 92 % |
| MP3 64 kbit/s | 0.033 (mean 0.042, p99 0.16) | 87.2 % | 98 % / 78 % | 88 % |

Re-encoding *moved* retrieval rather than costing it: **hit@1 rose** four to eight points in the maintainer's wording (two to four of fifty queries),
while zero-shot accuracy fell 0.3 – 0.6 points and, at 64 kbit/s, the stranger's hit@10 fell four. Why hit@1 rose is not known [inference: CLAP's
training audio is lossy, so a lossy clip may sit nearer it; untested]. What it means for a reader: **a retrieval number is a number about one
encoding of one library**, and a change in hit@1 smaller than about eight points is within what re-encoding the same clips does. The silence ESC-50
pads its clips with is not what moved the most-changed clips (clapping, sirens, church bells; their digital-silence share correlates 0.16 with the
change). The baseline in `golden/esc50/baseline.txt` is the WAV copy and stays so.

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

**The acoustic features share one transform where they can.** Spectral shape and tempo both read 1024-point frames at
hop 480 — the same frames, the same window — so `analysis::spectra` computes that transform **once per frame** and both
consume it: **9.44 ms → 5.11 ms per 4 s clip**. Loudness reads no spectrum, and key needs 8192 points for its chroma, so
neither joins the share; `log_mel` above reflect-pads to a different frame count and would be a different measurement, not
the same one twice.

### 3b. A long track is chunked, and pooled by loudness

A 6-minute track is 17.3 M samples; the model's window is 10 s. `chunk::Chunking` states the policy rather than leaving
it to the model's internal fusion — which works, but is not retrievable and would change every vector in an index without
the stamp changing.

| field | default | why |
|---|---|---|
| `window_s` | **10.0** | the model's own window, so a chunk is exactly one window with no padding |
| `hop_s` | **5.0** | half the window: every sample is inside two chunks, so a fade or a chord change on a boundary is fully present in one. A 6-minute track is **71 chunks**, which the guide states rather than hides because a chunk is one model call |
| `weight_by_loudness` | **true** | a plain mean weights a silent intro like the chorus, and CLAP embeds silence near the origin — pulling the whole track toward the null vector |

`chunks()` returns each window's start, length and **offset in seconds**, and the last chunk is clamped to reach the final
sample so a tail shorter than the hop is never dropped. `pool()` weights by each chunk's RMS, drops anything at or below
**−60 dB** entirely, and normalises — so a track that is silent everywhere has **no vector** rather than a zero one.

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
inference, not decoding, is what a large library waits on. **Measured end to end with the real graph** (`examples/semantic`,
M4 CPU, fp32, 280 distinct clips of about a second, decode included): **16.6 clips/s, 60 ms a clip** — **35 ms the tower
and 26 ms this crate's own mel spectrogram**, run one after the other — so a 5 000-clip library is about **five minutes**,
once. **A clip's cost is not constant:** 2 000 five-second 44.1 kHz recordings (`examples/goldenbench`, under a load average of
6.5) cost **105 ms** each, so the same library of those is about **nine minutes**; where the extra 45 ms goes — resampling and the
loudness, spectral, tempo and key passes over five times the audio — was not decomposed. The graph's input is a fixed ten-second
window, so a short clip costs what a ten-second one does and a longer one is chunked into several windows; a GPU's figure is still
unmeasured (`#36`).

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

**Four rules, two kinds.** Loudness and tempo **exclude**; the noise floor and key **penalise**:

| rule | kind | threshold | why |
|---|---|---|---|
| loudness | exclude | `loudness_tolerance_db` 6 dB | two sounds more than 6 dB apart are deliberately at different levels |
| tempo | exclude | `tempo_tolerance_bpm` **4 BPM** | **twice the estimator's ±2 BPM uncertainty** — below that, a gap may be measurement error |
| noise floor | penalise | `noise_floor_tolerance_db` 12 dB | a hissier take is a worse neighbour, not an impossible one |
| key | penalise | `compatibility` 0.5 / 0.85 | a modal piece sits anywhere, and key is the least reliable feature |

A project that wants clashing keys sets `compatibility.clash_penalty: 1.0` and the key rule stops penalising — the
number is the flag, so no separate switch is needed.

## Ask with reasons: `search_ranked`

`search` gives scores on CLAP's own scale. `search_ranked` gives **every result its own explanation**, and re-sorts by the
project's weights — which is what makes a weights file binding rather than decorative.

```rust
use bevy_siren::semantic::search::{Ranked, search_ranked};
use bevy_siren::semantic::{rank::Floor, weights::Weights};

let weights = Weights::default();
let request = Ranked {
    embedding: &embedding,
    anchor,                 // the asset whose acoustic features every candidate is judged against
    features: &features,    // by content hash, from the indexer's measurements
    k: 3,
    floor: Floor::default(),
    weights: &weights,
};
let answer = search_ranked(&index, &stamp, &query, &request)?;
for result in &answer.matches {
    println!("{} {:.3} — {}", result.hash, result.score, result.explain());
}
```

**One scale, so the weights mean something.** Every signal becomes an agreement in `[-1, 1]` — `+1` as alike as the
signal can say, `0` neither, `-1` as far apart as the rule already calls fatal:

| signal | raw value | why that number |
|---|---|---|
| semantic | the cosine, unchanged | it **is** already `[-1, 1]`, so no map can improve it |
| acoustic | `1 − 2·difference / tolerance`, clamped | the tolerance the filter refuses on maps to `-1` |
| tempo | the same shape, on `tempo_tolerance_bpm` | |
| key | `2·multiplier − 1` | the table's `1.0` is `+1`, its `0.5` neutral, its `0.0` is `-1` |

**The scale is signed on purpose.** A `[0, 1]` map was the first attempt and made every contribution positive, so
`Breakdown::strongest()` could never report what counted *against* a result — `explain()` was a score with decoration.

**A missing signal is not a disagreement.** An asset with no tempo estimated, or a key judged atonal, contributes nothing
and is marked `missing` — never a zero, which would read as *"consulted and disagreed"*. `explain()` says which:

```
semantic carried it (+0.420 from 0.42 raw at weight 1); acoustic counted against it (-0.250 from -0.5 raw); no value for tempo, key
```

**The hard rules run before the scoring.** A candidate beyond the loudness or tempo tolerance is excluded and never
scored, so `#27`'s rules are binding rather than advisory; `RankedAnswer` reports `excluded` and `dropped` separately so
an empty result can say which rule emptied it.

## The workflow: anchor, neighbours, and why

`semantic::tools` is the workflow the retrieval engine serves, as three calls an editor panel or an agent makes:

```rust
use bevy_siren::semantic::tools::{Session, Toolset};

let tools = Toolset::default();
let mut session = Session::new();
session.set_anchor(&index, chosen_hash)?;          // idempotent; one anchor per session, no global

let music = tools.compatible_music(&index, &session, None, 3)?;
let effects = tools.compatible_sfx(&index, &session, None, Some("footsteps"), 6)?;
let why = tools.explain_match(&index, chosen_hash, music.results[0].hash)?;

println!("{}", music.describe());   // "3 returned, 1 excluded by the acoustic rules, 2 near-identical variants"
println!("{}", why.explain());      // "semantic carried it (...); acoustic counted against it (...)"
```

**Two weightings, not one changed quietly.** An effect has no key and no tempo, so `Weights::effects()` moves the weight
those held onto acoustic agreement and keeps the total — a score from one table is comparable to the other's, and
`shares()` never reports weight spent on a signal that had no value.

**A variant is reported, not repeated.** Results are ordered first, then a result within `Variants::identical_below`
(0.99) of one already kept is left out **by name of the one it resembled** — so twelve takes of one footstep do not fill
the top twelve, and a caller still learns there were twelve.

**No anchor is a refusal with a remedy**: *"no anchor is set. Call `set_anchor(asset_id)` first, or pass one
explicitly"*.

## The tool server: `siren-tools`

The same calls, for an agent: an **MCP server over stdio**, in this crate behind `semantic`
([ADR 0006](../adr/0006-plugin-and-tool-server.md) for where it lives, [ADR 0007](../adr/0007-tool-surface.md) for the
tools, [ADR 0008](../adr/0008-tool-host.md) for the transport and the error mapping).

```sh
cargo run --release --features semantic --bin siren-tools -- <library-dir> [--model <file.onnx> --vocab <tokenizer.json>]
```

`<library-dir>` holds the index the indexer wrote (`indexer::INDEX_FILE`). With `--model` **and** `--vocab`, `search_music`
embeds a description with that graph; without them it answers `unavailable` and names the two flags, and the tools that need
only the index work either way. A client lists it like any stdio server:

```json
{ "mcpServers": { "siren": { "command": "siren-tools", "args": ["/path/to/library", "--model", "clap-text.onnx", "--vocab", "tokenizer.json"] } } }
```

| tool | does | idempotent |
|---|---|---|
| `search_music(description, k = 3, anchor?)` | rank the library by a description; a pure read, it **never** reads the session's anchor | yes |
| `set_anchor(asset_id)` | choose the asset the next questions are about; returns what the library knows of it | yes |
| `find_compatible_music(anchor?, k = 3)` | music that fits the anchor, with what was excluded and why | yes |
| `find_compatible_sfx(anchor?, category?, k = 6)` | effects from the same world; near-duplicates reported, not repeated | yes |
| `explain_match(asset_id, anchor, weighting = "music")` | every signal's raw value, weight and contribution for one pair; pass `"effects"` to explain a `find_compatible_sfx` result, or the weights will not add up to its score | yes |
| `audition(asset_id, bus = "Effects")` | **`unavailable`**: it needs a running app, and the server never owns playback | **no** |

**Every failure of a tool is a result a model can read**, not a protocol error it never sees: `{"error": {"code",
"message", "retryable"}}` with `isError: true`. `bad_argument` is `retryable: true` (the call was wrong; fix it and ask
again), and `refused`, `unavailable` and `internal` are `false` (the *state* must change, so retrying unchanged fails
identically). A bad call names **every** problem at once. `k` is a bound from 1 to 100, never a page.

**One server is one session**: the anchor belongs to the connection and is lost when it closes. The index is read once at
start-up and held in memory, so a long-lived process is the expectation — measured at **34 ms and 20.5 MB** for 10 000 assets,
then **0.02 ms** for `set_anchor` and `explain_match`, **5.3 ms** for a search (before the text encode) and **18 ms** for
`find_compatible_*`, which scores every candidate ([ADR 0008](../adr/0008-tool-host.md) has the table and the command).

The protocol is in `semantic::server`, so a host or a test drives it without a pipe: `Host::new(index, stamp, embedder)`,
then `host.handle(line)` for one JSON-RPC line in and at most one out.

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

## The floor: what two libraries say

`Floor` drops results below a cosine. Its default is **0.0**, which keeps everything. **The golden set measured two libraries and no positive
number works for both** — the decision is `docs/loop/needs-decision/default-floor.md`, and the default is unchanged.

It was chosen from six synthetic clips whose right and wrong score bands overlapped — *"no single threshold separates them; a floor
of 0.5 rejects every correct match"*. **On real recordings that is false.** [The golden set](golden-set.md) ran 2 000 of them: the
right answers' median is 0.468 against the wrong answers' 0.070, the bands overlap only in their tails, the weakest top result of any
of 50 descriptions scores 0.492 (0.405 when the same sounds are described in a stranger's words), and a floor on the top result tells a
description the library can answer from one it cannot with an AUC of 0.963 (0.885 in the stranger's):

| floor | top-1 kept | results per query (of 2 000) | precision | no-answer queries silenced |
|---|---|---|---|---|
| 0.00 (the default) | 90.0 % | 1 440.3 | 2.8 % | 0 of 16 |
| 0.30 | 90.0 % | 106.9 | 34.6 % | 0 of 16 |
| 0.40 | 90.0 % | 42.5 | 68.6 % | 7 of 16 |
| **0.45** | 90.0 % | **27.7** | **81.6 %** | **9 of 16** |
| 0.50 | 90.0 % | 16.5 | 90.4 % | 11 of 16 |

Read the table with its caveats. **It is ESC-50's, in the maintainer's wording**: in a stranger's, a floor of 0.45 already loses two top results and 0.50 loses nine. One library of
environmental recordings that overlaps the model's training data, sixteen no-answer queries — and the ones a floor cannot silence are the ones with a real neighbour (a gunshot finds
`fireworks` at 0.59).

**And the second library — 303 game sounds from Kenney's CC0 packs — answers differently.** Its right answers' median is **0.191** against 0.468, the weakest top result of an answerable description is **0.261**
(0.238 in the stranger's wording) against 0.492, and **a helicopter finds an `engine` at 0.490**, above the best result of 27 of the 29 answerable descriptions. The AUC of a floor on the top result is
**0.759** (0.808), against ESC-50's 0.963 (0.885) — the 95 % intervals over the descriptions overlap in the stranger's wording, so **the count carries the conclusion, not the AUC**: **13 of 29 descriptions have a best
result under 0.35 on Kenney (6 of 29 in the stranger's), 0 of 50 on ESC-50**, and no floor keeps every answer and silences every miss on either library. The `0.35` that is free on ESC-50 would have returned nothing for
those 13. **The largest floor that costs neither library an answer in either wording is 0.20, and at 0.20 it silences no no-answer query on either**; one high enough to silence half of them (0.30)
deletes answers. A floor *relative to the query* (each score less that query's median) is worse on game audio (AUC 0.626), and what helped in an exploratory test is on the gallery side — a hub, one
class that is the first result for 22 of 47 descriptions — and is not built (`golden-set.md`, "The second library"). The lesson that matters more than any number: **a cosine's scale belongs to a library —
synthetic tones understated a real match, environmental recordings overstate a game's — so a floor calibrated on one is calibrated on nothing for the other.**

**A project that wants a floor sets one from its own descriptions that have an answer** (`Weights::min_similarity`, or `Ranked.floor` per request): run them, take the lowest top result and stay under it — a
different writer lowered it by 0.02 on Kenney and 0.09 on ESC-50. **Not from descriptions that should find nothing**: on Kenney the strongest of those scored 0.490, above the top result of 27 of 29 answerable
descriptions, so a floor that silences them deletes the answers.

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
its value in its own doc comment. None is tuned against a person's judgement of fit — `#35`'s anchor pairs would supply it — and two
were argued from the six-clip measurement the golden set superseded (`acoustic` at 0.5, and the floor), so they are design defaults
under review, not findings. `drop_fraction` is 0.0 because a filter that hides the right answer is worse than a long list.

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
