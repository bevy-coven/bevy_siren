# Iteration 115 — eval: the baseline, the cue table, and the claims audit

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

**Nothing is red.** Every command exits 0 and the counts are iteration 114's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + **200** |
| `--features serde` | **209** |
| `--features semantic` | 109 lib + **517** |
| `--features mixer` | 6 lib + **211** |
| `--all-features` | 115 lib + **542** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large (max 16MB)`* from `examples/mixer` against the full umbrella) is not this crate's code.

## The cue table (`cues.yaml`)

All **34** cases are asserted by the matrix (each case's `evidence:` names its tests) and none is red; case 34 (iteration 114) is the only one whose observation is a sample the crate's mixer produces. Cases 1–33 observe numbers the crate writes.
**What the table cannot see**: no case concerns **how long a loop is**. Case 15 (*a layer change lands on the bar*) is computed from the crate's own clock; case 34 holds the gain of a loop and not its length.

## Half-done work

None. `git status` is clean at `4bfde69` (= `origin/main`). Every run directory has a `review.md` except `-78` (research only) and `-98` (one file written by `evalbench`), the two iteration 112 recorded. Open issues, re-read through `issue://`: `#32`–`#34` the editor's (built in `bevy_grimoire`; pinned 221 commits back),
`#35` waits on a person's anchor pairs, `#39` was decided not to ship (ADR 0005) and waits on the owner. **No `consumer` issue is open.**

## The claims audit — by `grep` and `read`

Iteration 114's top five put one item first and costed it: **an Ogg loop is not sample-exact on either path.** Re-reading what the repository says about it:

- `src/semantic/decode.rs:14-16` — *"`mixer` plays that padding as up to 20 ms of near-silence (median peak −69 dBFS, worst −35); the index would have ranked it, so only the index path cuts it."* The crate knew, cut it where a model would notice, and left it where a person would hear it.
- `docs/guide/semantic.md:160` — *"(`mixer` plays through `rodio` directly and still gets the padding.)"*
- `README.md`/`docs/guide/cues.md:67` — the documented sound is `assets.load("audio/step_01.ogg")`. **Ogg is the format Bevy enables by default**, so the format a loop is most likely to be in is the one that is longer than it declares.
- `docs/guide/music.md:20-22` — a layer *"keeps every layer phase-aligned — which is the whole point"*. Nothing in the crate or its tests holds a loop's **length**: each pass of a padded stem is longer than its bar by up to 20 ms, so the audible downbeat drifts from the crate's bar clock by one padding a pass,
  and a layer entry computed *on the bar* lands early against it — 120 ms after ten passes of a stem whose padding is 12 ms (arithmetic, from the fixture's 576 frames at 48 kHz; the measurement is `research.md`'s P1).
- **Both paths loop the decoder's output as it comes**: Bevy's `audio_output.rs:160-165` (`decoder.repeat_infinite()`, or `decoder.take_duration(duration).repeat_infinite()` when `PlaybackSettings::duration` is set) and the crate's `mixer/mod.rs::looped`. Neither asks how long the stream is.

**No test reads a loop's length.** Case 34's tests (iteration 114) read the gain of a loop of exactly one length, built from a `SamplesBuffer` that has no padding.

## Ranked (`plan.md` has the full list)

1. **Bugs and wrong output — an Ogg loop is longer than its stream, on both paths.** A gap of up to 20 ms of near-silence at every wrap (above the ~3 ms gap threshold of normal hearing) and a bar drift; measured in iteration 112 on 59 of 209 real files; the route is found.
2. Everything else in iteration 114's top five: a person listening (not mine); the hubness measurement (experimental); the floor decision (the owner's); `play`'s per-play copy (performance).

## After (`matrix-final.log`, the second full run; the first was red on clippy alone — see `review.md`)

Every command exits 0.

| command | before | after |
|---|---|---|
| `cargo test` | 6 doc + 200 | 6 doc + 5 lib + **203** |
| `--features serde` | 209 | 5 lib + **212** |
| `--features semantic` | 109 lib + 517 | **114** lib + **521** |
| `--features mixer` | 6 lib + 211 | **16** lib + **216** |
| `--all-features` | 115 lib + 542 | **125** lib + **547** |
| `cargo test --doc` / readme | 6 / 1 | 6 / 1 |
| clippy `-D warnings` / fmt / rustdoc `-D warnings` | clean | clean |
| `cargo build --examples` (default and `--all-features`) | ok | ok (the same linker warning) |

**New tests:** 5 in `src/ogg.rs` (the page walk: one well-formed, 17 refusals as rows, the window's edges, a second rate and channel count, `loop_length`); 5 in `src/mixer/mod.rs` (a loop cut to its declared length, 108 cells of `TakeDuration` exactness, through a real `Player` and bus `Mixer`, which voices are cut, a source that starts with an empty span); 4 in `tests/ogg_loops.rs`
(told in the frame it is played, told when the asset arrives, nothing told for six kinds of sound, the duration through `rodio` itself with a length assertion); 1 real-device test (`tests/mixer.rs::an_ogg_loop_on_the_real_mixer_sounds_until_it_is_stopped`).
Each went red first (`evidence/before/`), and five mutations were run (`ogg-mutation-no-one-stream-guard.txt`, `ogg-mutation-no-window.txt`, `mixer-mutation-trim-ignored.txt`, `real-mixer-ogg-loop-without-openspan.txt`, `ogg-loops-guard-on-symphonia.txt`).

**The cue table is 36 cases.** Case 35 (*an Ogg loop sounds*) and 36 (*it is as long as the stream declares*) are new and green. Case 35 exists because the audit's question was the wrong one: *"does any test read a loop's length?"* (none) was answered, and **"does any test loop an Ogg at all?"** (none) was not asked until the first measurement showed the loop empty.

**Render:** 21 of 21 WAVs byte-identical to iteration 114's; `mix.wav` −21.5 LUFS, −7.5 dBFS true peak; the standard spectrogram read.
