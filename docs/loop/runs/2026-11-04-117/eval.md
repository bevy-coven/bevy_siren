# Iteration 117 — eval: the baseline, the cue table, and the claims audit

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

Tree clean at `e0e8a86` (= `origin/main`); open issues `#32`–`#35` and `#39` (re-read through `issue://`), **none labelled `consumer`**. **Nothing is red.** Every command exits 0 and the counts are iteration 116's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + 5 lib + **208** |
| `--features serde` | 5 lib + **217** |
| `--features semantic` | **114** lib + **526** |
| `--features mixer` | **16** lib + **222** |
| `--all-features` | **125** lib + **553** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large`* from `examples/mixer` against the full umbrella) is not this crate's code; the baseline of iterations 115 and 116 carried it too.

## The cue table (`cues.yaml`)

All **37** cases are asserted by the matrix (each case's `evidence:` names its tests) and none is red. **What the table cannot see**: no case concerns **what a play of bytes the decoder refuses does on Bevy's path** — the build a consumer gets. Cases 10 – 13, 25 and 37 pin the refusals the crate can name (no cue, no handle, no bus, no ears, a failed load, bytes the *crate's mixer* refuses) and every headless test's bytes are either a default (Uuid) handle's *none* or placeholders behind a fake `Output`, so the one state that crashes a game is a state no case builds.

## Half-done work

None. `git status` was clean at `e0e8a86` (= `origin/main`); `runs/2026-11-04-116/` has its `review.md` (GREAT, GREAT, GREAT) and its log entry.

## The claims audit — by `grep` and `read`

Iteration 116's lesson was *"a test fixture that is always in the happy state cannot see the other states"*. This iteration asked it of the sentences about **bytes**: *which test builds a file whose bytes are not audio, on the path a consumer runs?*

| sentence | where | held by | verdict |
|---|---|---|---|
| *"A sample the decoder refuses is reported as a `Refused` with the key, so a project learns which file to deliver rather than hearing nothing"* | `playback.rs:8-10` | `tests/mixer.rs` (a **fake** `AudioOutput` that returns `None`) | true on the crate's mixer; **on Bevy's path: `Played`, then a panic in the same frame** (`evidence/before/`) |
| *"`Decoder::new` returns `UnrecognizedFormat` for anything else, which the backend warns about rather than panicking on"* | `Cargo.toml:158-159` | nothing | true of the mixer's backend; **Bevy's own `AudioSource::decoder` is `….build().unwrap()`** (`audio_source.rs:96-101`) |
| *`MissingSound` … "the undecodable case [is found] when the bytes first load"* | `play.rs:127-129` | the same fake | on Bevy's path nothing is found, and the process ends |
| *"a key whose bytes the decoder refused" is refused* | `docs/guide/cues.md:147-148` | the same fake | same |
| *"Every play answers exactly one of `Played`/`Refused`. … nothing plays on a refusal."* | `AGENTS.md` | `tests/play.rs` (none of its bytes are bytes) | **false for six kinds of file on the default build**: `Played`, no `Refused`, a panic |
| the row *"a loop of bytes that are not audio"* | `tests/ogg_loops.rs:98` | a headless app that never runs Bevy's decoder | passes, and builds the state that crashes a game |
| *"a headless app … every system works without a device, and a voice is still a `Voice` with a `Played` behind it"* | `lib.rs:135-137` | the headless tests | true, and **it is why this was never seen**: no device, no `play_queued_audio_system` work, no decoder call |

**The probe** (`evidence/before/undecodable-probe-on-the-unchanged-tree.txt`; the real `AudioPlugin` on this Mac's device, the real `AssetServer`, the file loaded first, one `Play`, one process per file): a Git LFS pointer, an empty file, a 40-byte cut of a real Ogg, an **Ogg Opus** file, an ADPCM WAV and a µ-law WAV — **six `Played`, no `Refused`, six panics at `audio_source.rs:101:14`, in the first update after the play**. The two good controls sounded and ended.

## Ranked (`plan.md` has the table)

1. **Bytes that load and do not decode crash the game on its default path, and the crate says it refuses them.** Probed on the real stack; deterministic; the commonest delivery mistakes.
2. The layers of a score starting apart (iteration 116's carve-off; 3 of 100 warm-cache starts).
3. A `Play::on` for a vanished owner leaves an ownerless loop (read, not run).
4. A person listening (not mine).

## After (`matrix-final.log`, the last of this tree's runs)

The matrix ran more than once on this tree and the log file is the last (the earlier run, on the first design — a memo in each of two systems — was also green and is overwritten by the same file name; it carried the `unused_mut` warning described in `review.md`).

| command | baseline | after |
|---|---|---|
| `cargo build` | ok | ok |
| `cargo test` | 5 lib + 208 | **9 lib** + **213** |
| `--features serde` | 5 lib + 217 | **9** + **222** |
| `--features semantic` | 114 lib + 526 | **118** + **531** |
| `--features mixer` | 16 lib + 222 | **20** + **227** |
| `--all-features` | 125 lib + 553 | **129** + **558** |
| `cargo test --doc` / `readme::` | 6 / 1 | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean | clean |
| `cargo fmt --check` / `cargo doc` (`-D warnings`) | clean | clean |

**New: nine tests.** Five integration (`tests/undecodable.rs`): bytes the decoder refuses are refused with the cause (six rows); **the crate refuses exactly the bytes `AudioSource::decoder()` panics on** (six undecodable rows and four good fixtures, in every feature set); bytes that arrive after the play are refused in the frame they arrive;
the control — a decodable sound is played, whenever its bytes arrive; an edit to the file is asked again (including one that keeps the length and the first 16 bytes). Four unit (`src/sounds.rs`): a buffer is asked once and an edit again; the memo forgets assets that are gone; a hint comes from the first bytes only;
**a sound is decoded once by the crate however often it is played.** Two fixtures added (`tests/fixtures/opus.ogg`, `adpcm.wav`, from `ffmpeg`). **One existing test changed**: `tests/ogg_loops.rs::anything_but_a_looping_ogg_that_declares_its_length_is_told_nothing` lost its *"bytes that are not audio"* row (the state it built crashed a game) and its FLAC and MP3 rows
now exist only where the build decodes them. The only remaining warning is the `example "mixer"` linker one, which the baseline had.

**The cue table is 38 cases** (case 38 new), none red. **Render**: 21 of 21 WAVs byte-identical to iteration 116's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix-spectrogram.png` read. `perfcheck` ×3 in `evidence/after/perfcheck.txt`; the system's own cost in `perf.md`.
