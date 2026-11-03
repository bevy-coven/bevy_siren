# 0003 — Acoustic features: what lives in Rust, what needs a sidecar

- **Status:** accepted
- **Date:** 2026-11-03
- **Issue:** [#4](https://github.com/bevy-coven/bevy_siren/issues/4)
- **Blocks:** [#21](https://github.com/bevy-coven/bevy_siren/issues/21),
  [#22](https://github.com/bevy-coven/bevy_siren/issues/22),
  [#23](https://github.com/bevy-coven/bevy_siren/issues/23),
  [#24](https://github.com/bevy-coven/bevy_siren/issues/24),
  [#25](https://github.com/bevy-coven/bevy_siren/issues/25)
- **Evidence:** [`loop/runs/2026-11-03-77/research.md`](../../loop/runs/2026-11-03-77/research.md)

## The question

Six acoustic features decide whether the ensemble can tell two tracks apart that embed close but sound different:
loudness, true peak, spectral centroid, spectral rolloff, noise floor, tempo, and key. Which of them can this crate
compute in Rust, and which need the Python sidecar?

The decision has to be made before `D-01` because it fixes the crate's dependency list, and that list is **closed by a
test** (`tests/leaf.rs::the_dependency_list_stays_closed`, `ALLOWED_DEPS`).

## What was measured first

### Loudness and true peak: `ebur128` 0.1.11 is pure Rust

The crate that is *named* like a C library is not one. Read from `ebur128-0.1.11.crate` directly, not from its
description:

| | |
|---|---|
| `src/` | **nine `.rs` files**, including `filter.rs`, `true_peak.rs`, `interp.rs`, `history.rs` |
| `.c` files in the package | 8, **all under `capi-test/`, `tests/c/`, and one `assets/ebur128.h`** |
| `build.rs` | runs only under `#[cfg(feature = "c-tests")]`, whose job is to compile the **reference C** so the Rust port can be compared against it |
| default features | **none** — `c-tests`, `capi` and `internal-tests` are all off |
| normal dependencies | `bitflags`, `dasp_frame`, `dasp_sample`, `smallvec` |
| `cc` | a **build dependency, optional**, reachable only through `c-tests` |

So with `default-features = false` there is no C compiler, no `cc`, and no FFI in the built artifact. The C in the
package is a **conformance oracle**, which is exactly what #21's acceptance criteria ask for.

### The short-clip trap, measured on the reference

This is #21's own second criterion — *"a clip shorter than the standard's gating block is handled explicitly, not
reported as −70 LUFS by accident"* — measured with `ffmpeg`'s `ebur128` (the **judging** tool, which this ADR does not
promote into a dependency):

| file | length | peak | integrated |
|---|---|---|---|
| 1 kHz tone, 0.1 amplitude | 3.0 s | **−20.0 dBFS** | **−23.0 LUFS** |
| 1 kHz tone, 0.5 amplitude | **0.2 s** | **−6.0 dBFS** | **−70.0 LUFS** |
| seeded white noise, 0.25 amplitude | 3.0 s | 1.0 dBFS | −8.9 LUFS |

The 0.2 s clip is **14 dB louder at its peak** than the first and reports **47 LU lower**. Nothing is wrong with the
reference; a gated measurement has no gating block to integrate. A sound-effect library is mostly sub-0.4 s files, so
this fires on day one.

### And the Rust crate answers differently again, which is the finding that shapes #21

The same three files through `ebur128` 0.1.11 (`Mode::I | Mode::TRUE_PEAK`, `default-features = false`):

| file | `ffmpeg` integrated | `ebur128` `loudness_global()` | `ebur128` true peak |
|---|---|---|---|
| 3.0 s tone | −23.0 LUFS | **−23.01 LUFS** | −20.00 dBTP |
| **0.2 s tone** | −70.0 LUFS | **`-inf`** | −6.02 dBTP |
| 3.0 s noise | −8.9 LUFS | **−8.92 LUFS** | 0.82 dBTP |

Two long files agree to **0.01 LU**, so the gating and K-weighting are not in question. The short clip is where the two
differ, and **neither answer is usable as a measurement**: ffmpeg reports the floor as if it were a level, and
`ebur128` reports `-inf`, which is not a number a ranking can compare.

The crate is explicit about this in its own source: `gated_loudness_multiple` returns `-f64::INFINITY` on
`NoBlocksAboveThreshold`, and `relative_threshold()` returns exactly `-70.0` when there are no blocks — the **same
number** a genuine −70 LUFS passage produces. `#21` therefore cannot simply store `loudness_global()`: it must **count the
gating blocks** (`gating_block_count_and_energy`, which the crate exposes) and represent "too short to measure" as its
own state rather than as a level. That is a design decision #21 owns, and this ADR fixes only the measurement it rests
on.

## Decision

**All six features are computed in Rust. No sidecar is built for any of them.**

| Feature | Decision | Why |
|---|---|---|
| integrated loudness (LUFS) | **`ebur128`, `default-features = false`**, with a **gating-block count carried alongside** | pure Rust (verified: a clean build in **1.66 s**, no `cc`, no C object), and it agrees with `ffmpeg` to **0.01 LU** on measurable clips |
| true peak (dBTP) | **`ebur128`**, `Mode::TRUE_PEAK` | same crate; 4× oversampling through `InterpF<12, 4, _>`, per BS.1770 Annex 2 |
| spectral centroid | **hand-rolled** over the existing FFT | `mel.rs` already has a 1024-point radix-2 `fft_in_place`; a centroid is a weighted mean of bin frequencies |
| spectral rolloff | **hand-rolled**, same FFT | a cumulative-energy threshold over the same spectrum |
| noise floor | **hand-rolled**, same FFT | a low percentile of frame energy |
| tempo | **hand-rolled onset autocorrelation**, with a **stated accuracy floor** | no pure-Rust estimator with a published figure exists; see below |
| key | **out of scope for the Rust path** | see below |

### Why not `rustfft` for the spectra

`rustfft` 6.4.1 is correct and fast, but it would be the crate's **seventh** dependency and buys nothing this crate does
not already have: `mel.rs` computes a 1024-point radix-2 transform in thirty lines, and it is **tested against CLAP's own
extractor to 0.0545 dB** (`runs/2026-11-03-76`). Adding a general FFT to satisfy three statistics computed on the same
bins is a dependency that does not earn its place.

### Tempo: an accuracy floor, or it does not ship

No mature pure-Rust tempo estimator reports a published accuracy figure, and `aubio` is C and would itself be a
dependency decision. So tempo is **in the Rust path with a floor that is a test, not a claim**:

- **Method:** onset-strength autocorrelation over 40–200 BPM, seeded and deterministic like everything else here.
- **Floor:** a **±2 BPM** estimate at 80–160 BPM on a metronomic click corpus of at least five tempi, **or the feature
  is removed from the ranking**. `#24`'s acceptance criteria carry this number, and `#27` must treat a tempo outside the
  floor as **unknown**, which its criteria already require (*"a feature that is unknown never excludes an asset"*).
- **Why ±2 BPM:** it is the resolution at which a tempo mismatch stops changing which track "fits" — at 120 BPM, ±2 BPM
  is ±1.7 %, below the just-noticeable difference for tempo in a rhythmic layer. **Design default**, stated because no
  source gives this number for a game's music transitions.

### Key: out of scope for the Rust path

Key detection needs a chroma representation and a key-profile correlation, and no pure-Rust implementation reports a
usable accuracy figure. Rather than ship a feature that is wrong 40 % of the time and let `#27` penalise on it, **key is
declared out of the Rust path**. `#25` stays open; `#27`'s key rule must be a no-op when the key is unknown, which is the
behaviour its own criteria require for any unknown feature. A sidecar for key alone is not justified by one feature.

## The input assumptions, stated so the numbers mean something

| | |
|---|---|
| sample rate | **48 000 Hz**, the model's rate (`semantic::MODEL_SAMPLE_RATE`); a source file is resampled first, as `semantic::resample` already does |
| channels | **mono** for every feature; a stereo file is downmixed before measurement, and the downmix is the existing buffer path, not a second one |
| loudness window | **400 ms gating blocks, 75 % overlap** — `needed_frames` is documented as *"400 ms of audio at initialization, and 100 ms after the first block (75% overlap as specified in the 2011 revision of BS1770)"* |
| gating | absolute **−70 LUFS** (`relative_threshold()` reads −70.0 on silence, asserted in the crate's own test) and relative **−10 LU** (`let relative_gate = -10.0` in `history.rs`) |
| K-weighting | a two-stage IIR at **f0 = 1681.97 Hz, G = 3.9998 dB, Q = 0.70718** (the high shelf) and **f0 = 38.14 Hz, Q = 0.50033** (the RLB high-pass), read from `filter.rs::filter_coefficients` |
| true peak | **4× oversampling** through a 12-tap interpolator — `InterpF<12, 4, _>` in `true_peak.rs` |
| channel handling | the crate's `Channel` map weights each channel; mono is weight 1.0, which is the only case this crate uses |

The analyser is fed a decoded mono buffer from `semantic::decode`, already at 48 kHz, so no second resampler or downmixer
is introduced.

## What this costs

- **One new dependency**, `ebur128`, and it needs `ALLOWED_DEPS` in `tests/leaf.rs` extended **in the same commit**, with
  the sentence that argues for it. That is the ratchet working as designed.
- **Five features are hand-rolled**, so each needs its own test against a reference: `ffmpeg`'s `ebur128` for loudness
  and true peak, and synthetic signals with known spectra for centroid, rolloff and noise floor.
- **Tempo carries a risk**: if the ±2 BPM floor is not met, `#24` ships nothing. That is the correct outcome — a tempo
  estimate outside the floor is a wrong number that #27 would exclude real matches on.

## Rejected alternatives

| Option | Why not |
|---|---|
| **(b) sidecar for tempo and key** | Adds a process boundary, a transport and a second language to a crate whose only model dependency is already an in-process ONNX runtime. Chosen only if the tempo floor fails and key later earns its own evidence. |
| **(c) everything in the sidecar** | Contradicts ADR 0002, which established in-process inference, and moves five statistics that need no model across a process boundary. |
| **`aubio`** | C, and the dependency audit is the point of this ADR. |
| **`rustfft`** | A seventh dependency for a transform this crate already has, tested to 0.0545 dB. |
| **Promoting `ffmpeg`'s `ebur128`** | Explicitly forbidden by #4: it is the judging tool. The loop's own rule that a measuring instrument is not a library applies. |

## Promotion rule: when the sidecar becomes justified

A sidecar is built **only if one of these is measured**, not predicted:

1. The tempo floor fails **and** tempo turns out to matter to the ensemble's ranking (measurable once `V-01`'s labelled
   set exists).
2. A model `S-05` approves needs operators outside `ort`'s coverage.
3. Key detection earns its own evidence that it improves ranking enough to pay for a second process.

**Not a trigger:** the absence of a Rust implementation. That is an argument for stating a floor and testing it, which is
what this ADR does for tempo.
