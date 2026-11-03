# Iteration 77 — research: acoustic features, and which crate computes them

The item is [#4](https://github.com/bevy-coven/bevy_siren/issues/4), a p0 ADR that blocks five other issues. Its
deliverable is `docs/adr/0003-acoustic-features.md`; this file is the evidence behind it.

## Sources

| Source | What it reports |
|---|---|
| **ITU-R BS.1770-4** (the standard itself) | K-weighting = a +4 dB high shelf at ~1682 Hz feeding an RLB high-pass at ~38 Hz; gating blocks of 400 ms with 75 % overlap; absolute gate −70 LUFS, relative gate −10 LU; true peak by ≥4× oversampling |
| **EBU R128** | the −23 LUFS target for broadcast, and the same gating; a game mix is mastered lower, which is why this crate's own judging target is not −23 |
| `ebur128` 0.1.11, `src/filter.rs::filter_coefficients` | `f0 = 1681.974450955533`, `G = 3.999843853973347`, `Q = 0.7071752369554196`; then `f0 = 38.13547087602444`, `Q = 0.5003270373238773` |
| `ebur128` 0.1.11, `src/ebur128.rs` | `needed_frames` = *"400 ms of audio at initialization, and 100 ms after the first block (75% overlap as specified in the 2011 revision of BS1770)"* |
| `ebur128` 0.1.11, `src/history.rs:319` | `let relative_gate = -10.0;` |
| `ebur128` 0.1.11, `src/true_peak.rs` | `InterpF<12, 4, _>` — 4× oversampling through a 12-tap interpolator |
| `ebur128` 0.1.11 package listing | nine `src/*.rs`; 8 C/H files, all under `capi-test/`, `tests/c/`, `assets/ebur128.h`; `build.rs` gated on `#[cfg(feature = "c-tests")]`; features `c-tests`, `capi`, `internal-tests`, **all off by default** |
| `ebur128` 0.1.11, crates.io index | normal deps `bitflags`, `dasp_frame`, `dasp_sample`, `smallvec`; `cc` is a **build** dep, optional |
| `ffmpeg` 9.0.2 `ebur128` filter | the judging tool, used below to produce the reference numbers — **not** promoted to a dependency |
| this crate, `src/semantic/mel.rs` | a 1024-point radix-2 `fft_in_place`, tested against CLAP's extractor to **0.0545 dB** (`runs/2026-11-03-76`) |
| this crate, `tests/leaf.rs:35` | `ALLOWED_DEPS = ["bevy", "serde", "ron", "rodio", "ort", "rubato"]` — the closed list |

## The technique, in the sources' terms

Loudness is not a level; it is a **gated energy average over a K-weighted signal**. The K-weighting is a two-stage IIR
that de-emphasises bass and lifts the presence band, approximating the equal-loudness contour at conversational level.
The signal is then cut into 400 ms blocks overlapping by 75 %, each block's mean square energy becomes a loudness, blocks
below an **absolute** −70 LUFS are discarded, then the mean of the survivors defines a **relative** threshold 10 LU below
it, and blocks below *that* are discarded too. The integrated loudness is the mean of what remains.

The consequence, and the reason #21 exists: **a clip shorter than one block has no block to keep**, so it reports the
floor — −70 LUFS — regardless of how loud it is. Measured on the reference implementation:

| file | length | peak | integrated |
|---|---|---|---|
| 1 kHz tone, 0.1 amplitude | 3.0 s | −20.0 dBFS | **−23.0 LUFS** |
| 1 kHz tone, 0.5 amplitude | **0.2 s** | **−6.0 dBFS** | **−70.0 LUFS** |
| seeded white noise, 0.25 amplitude | 3.0 s | 1.0 dBFS | −8.9 LUFS |

The 0.2 s clip peaks **14 dB louder** and reads **47 LU lower**. Any sound-effect library fires this on day one.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| gating block | 400 ms, 75 % overlap | BS.1770-4; `ebur128` `needed_frames` |
| absolute gate | −70 LUFS | BS.1770-4; `ebur128` `relative_threshold()` |
| relative gate | −10 LU | BS.1770-4; `ebur128` `history.rs` |
| K-weight shelf | 1681.97 Hz, +4 dB, Q 0.707 | BS.1770-4; `ebur128` `filter_coefficients` |
| K-weight high-pass | 38.14 Hz, Q 0.500 | BS.1770-4; `ebur128` `filter_coefficients` |
| true peak | 4× oversampling | BS.1770-4 Annex 2; `ebur128` `InterpF<12, 4, _>` |
| broadcast target | −23 LUFS | EBU R128 (**not** this crate's target; a game mix sits lower) |
| tempo floor | **±2 BPM at 80–160 BPM** | **design default** — no source gives this for a game's music transitions |

## What good sounds like

A correct implementation reproduces the reference **to 0.1 LU**, because that is the tolerance #21 sets and the standard
is unambiguous enough to meet it. On a 3 s 1 kHz tone at 0.1 amplitude it reads **−23.0 LUFS** and **−20.0 dBFS**, and
it reads them from the buffer alone with no audio device. A wrong implementation is not obviously wrong: it is a few
tenths out because the K-weighting coefficients were approximated, or it is **−70 on a short clip** because the gating
was copied without the short-clip decision. The second is the one a person would notice, and it is the one a test must
pin by name.
