# Iteration 112 — research: do the formats the scan calls audio decode?

**The question.** `library::AUDIO_EXTENSIONS` is `wav, ogg, oga, flac, mp3` and its doc calls them *"the formats the crate can decode"*;
`indexer`'s error message says *"a library of `.mp3` files is indexed"*; `decode.rs`'s doc says rodio *"decodes wav, vorbis and flac
through the features this crate enables"*; the guide (my own sentence, iteration 110) says *"a folder of WAV, Ogg or FLAC files"*. The
crate's `rodio` is given `["playback", "wav", "vorbis"]`. Those cannot all be true, and **no test had a file that was not a WAV** — so
nobody had run any of them. What does each claim say, what does each format's specification say, and what would a right answer look like?

## Sources

| # | source | what it gives, and the number |
|---|---|---|
| S1 | `~/.cargo/registry/src/*/rodio-0.22.2/Cargo.toml` (vendored) | **The decoders are symphonia's, and a feature is a format.** `wav = [symphonia-wav, symphonia-pcm]`, `vorbis = [symphonia-ogg, symphonia-vorbis]`, **`flac = ["symphonia-flac"]`, `mp3 = ["symphonia-mp3"]`** — all four pure Rust (`symphonia 0.5.5`, no C). With `default-features = false` and the list the crate has, `Decoder::try_from` has no FLAC or MP3 reader registered, and says *"The format of the data has not been recognized."* |
| S2 | `symphonia-bundle-mp3-0.5.5/src/common.rs` (vendored) | A Layer III frame is **576 × granules** samples: 1152 for MPEG-1 (two granules), 576 for MPEG-2/2.5. So an MP3 decodes to a length that is a whole number of frames, and a 0.5 s file is 24 000 frames **± one frame** of encoder padding — the tolerance the tests use (±1 200), and the reason an MP3 is judged by pitch and level, not by sample |
| S3 | RFC 9639, *Free Lossless Audio Codec (FLAC)*, §8.2 (STREAMINFO) | The 34-byte block, in order: **16** min block size, **16** max block size, **24** min frame size, **24** max frame size, **20 bits sample rate**, **3 bits channels − 1**, **5 bits bits-per-sample − 1**, 36 total samples, 128 MD5. The rate therefore starts at byte **10** of the block and ends in the high nibble of byte **12** |
| S4 | Xiph.Org, *Vorbis I specification*, §4.2.2 (identification header) | After the 7-byte packet marker (`\x01vorbis`): **`vorbis_version` 32 bits, `audio_channels` 8 bits, `audio_sample_rate` 32 bits** (little-endian), then three 32-bit bitrates. Channels are at offset **4** from the end of the marker; the rate at **5 – 8** |
| S5 | Gregory, *Game Engine Architecture*, §14.3.2.3 "Digital Audio Formats and Codecs" (home-still corpus; the index carries no DOI) | The formats a game meets: uncompressed PCM, and lossy compressed ones — Ogg Vorbis, MP3, AC-3, DTS — with the note that lossy schemes give up fidelity for size. The case for a list of *five* extensions being the right list: it is what a library actually contains |
| S6 | Jacobellis, Cummings & Yadwadkar, *Machine Perceptual Quality: Evaluating the Impact of Severe Lossy Compression on Audio and Image Models*, 2024, DOI `10.48550/arxiv.2401.07957` (home-still / OpenAlex) | Lossy compression changes what perception models do, **by codec and by severity**, and not always for the worse. The reason to *measure* what MP3 costs retrieval rather than assume it is negligible — the paper does not report a CLAP number, and **no source in the corpus does** (four searches: codec robustness of audio embeddings, MP3 and audio classification, encoder delay, symphonia conformance — one relevant hit, S5) |
| S7 | `tests/fixtures/tone*.{flac,mp3,ogg}`, made with `ffmpeg 9.0.2` (`flac`, `libmp3lame`, native `vorbis`) | **The independent reference for S3 and S4.** A header built by hand from the same reading of the specification as the parser would agree with it whether or not that reading is right; a file an unrelated encoder wrote does not. Two rates (48 000, 44 100) and two channel counts, so a wrong shift cannot hide in a nibble |

## The technique

Check each claim against **a real file of its format**, in three places — (1) the **header** the scan reads (`read_header`: rate and channels
without a decoder), (2) the **decode** (`decode`, rate, channels, length, pitch, level), and (3) the **index** (a folder of every format
builds) — and tie the list to the decoder so they cannot drift: a test that takes each entry of `AUDIO_EXTENSIONS`, requires a fixture
for it, and decodes it through the model path.

## Parameters

| parameter | value | source |
|---|---|---|
| FLAC sample-rate field | 20 bits at STREAMINFO byte 10, high nibble of byte 12 | S3 |
| FLAC channels | `((s[12] >> 1) & 7) + 1` | S3 |
| Vorbis channels / rate | marker + 4 / marker + 5..8, little-endian | S4 |
| MP3 frame | 1152 samples (MPEG-1) | S2 |
| decode tolerance, lossy files | ±5 % of the length (24 000 ± 1 200); cycles within 218 – 221 of 220; amplitude within 12 % | **design default** — wide enough for a codec's padding and noise, narrow enough that a missed 16-bit scale, a doubled channel or a wrong rate is far outside it |
| dead band for counting cycles | ±0.03 (a quarter of the 0.125 peak) | **design default**; a plain crossing count read 231 for a correct 220-cycle Vorbis tone |

## What good looks like

- **FLAC:** bit-exact. The decoded samples are the 16-bit values the generator wrote (RMS error from the ideal sine ~1 × 10⁻⁵, asserted
  < 1 × 10⁻⁴), 24 000 frames exactly, 220 plain crossings. And at the *index* level a FLAC library gives **the same similarities as the same
  clips as WAV** — not "close": equal.
- **MP3 and Ogg Vorbis:** the right rate and channels, a length within a frame or a block, the right pitch, the right level — and, against an
  independent decoder (`ffmpeg`) over real files, **similarities that agree to about the pipeline's own noise** (iteration 110: 5 × 10⁻⁴ at
  48 kHz).
- **The header:** 48 000 and 44 100, 1 and 2 channels, from both Ogg and FLAC, as the encoder wrote them.

## A prediction, recorded before it is measured

**What MP3 at 128 kbit/s should cost retrieval.** MP3 low-passes near 16 kHz at that rate, and CLAP's mel filterbank stops at 14 kHz, so the
dominant loss falls outside what the model looks at; the rest is quantisation noise in the mid band, which the model sees as a slightly
noisier version of the same sound. **Prediction: ESC-50's hit@1 (90 % on 50 descriptions) moves by at most one query (±2 points), zero-shot
accuracy by at most 1 point, and a typical clip's similarity to a description by less than 0.01.** If the measurement is outside that, the
prediction was wrong and the guide will say how. (64 kbit/s is also run, as the worst case a game would ship.)

## What this does not settle

Whether symphonia's MP3 decoder is bit-exact to a reference decoder (it need not be: the standard allows a tolerance, and the metric is the
embedding, not the sample); gapless playback and loop points (a different feature); and any format the scan does not list (Opus, AAC, AIFF:
not claimed, so not tested).
