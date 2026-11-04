# Iteration 120 — eval: the baseline, the cue table, and the claims audit of the front door

## Baseline on arrival (`baseline-matrix.log`, the matrix script from iteration 112, run on the tree at `0a0e359`)

**Nothing was red.** The tree was clean at `0a0e359` (= `origin/main`); open issues are `#32`–`#35` and `#39`, none labelled `consumer`.

| command | result |
|---|---|
| `cargo build` | green |
| `cargo test` | 9 lib + **226** + 6 doc, green |
| `cargo test --features serde` | 9 lib + **235** + 6 doc |
| `cargo test --features semantic` | 118 lib + **544** + 6 doc |
| `cargo test --features mixer` | 20 lib + **241** + 6 doc |
| `cargo test --all-features` | 129 lib + **572** + 6 doc |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and all features), `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean; the only warning is the known `example "mixer"` linker one |

## The cue table (`docs/loop/cues.yaml`)

40 cases, **none red** (the suite that holds them is the matrix above). No case is about the README's own code: that is the gap this iteration's audit found.

## The claims audit: the front door, by running it

Iteration 118 ran the documented use of a handle the crate hands out, and 119 the documented ways a game hands the crate an entity. This one runs **the first thing a reader copies**: the README's Quick start, character for character, and then every command the README tells them to type. (118's probe had already shown the keys; this is the whole block, pasted and run, on the tree as it is today.)

| claim | where | what I ran | result |
|---|---|---|---|
| The Quick start block is a program that plays a footstep | `README.md:37-70`, `src/lib.rs:10-43` | the block pasted **verbatim** as an example of this crate; `cargo build`; then run seven seconds in a real window with the real output device (`evidence/before/readme-as-written.txt`) | **it plays nothing.** The compiler says `function walk is never used` and `function hear is never used`: `main` adds neither. The console shows no line from this crate, and one from Bevy: `ERROR bevy_asset::server: Path not found: …/assets/audio/step_01` (and `step_02`) |
| *"`sounds.load_table(&assets, &cue_table, "audio")`"* with the keys `"step_01"`, `"step_02"` | the same block | the same run, with `assets/audio/step_01.ogg` and `step_02.ogg` on disk | **the keys are not files**: `load_table` asks the server for `<dir>/<key>`, i.e. `audio/step_01`, and the file is `audio/step_01.ogg`. 118's probe: keys as written → every play refused (`failed to load: Path not found`); keys with the extension → five plays of five |
| `fn hear` — *"Which recording the player heard, and why it was chosen"* | the same block | the block's `hear`, scheduled as its comment implies, one play, headless (`evidence/before/hear-probe.rs.txt`) | **it writes a second log entry for every play, stamped `0.00s`**: `0.30s played …` (the crate's) and `0.00s played …` (`hear`'s copy, `at_s: 0.0` typed by hand). It prints nothing: a reader sees nothing |
| *"`step_03` sends you to `audio/step_03.wav`, which is the file to fix"* | `docs/guide/cues.md:147-149` | read against `load_table` (`format!("{dir}/{key}")`) | **false**: the key is the file's name under the directory, so a refusal naming `step_03` sends you to `audio/step_03`. The sentence two lines below (`step_3.wav` → `…/audio/step_3.wav`) is true, because that key has its extension |
| *"The README's quick start is the crate-root doctest (`tests/readme.rs` fails if the two drift)"* | `README.md:72` | read `tests/readme.rs`, `Cargo.toml` | **it compares two copies of the same text**, and **only with `--features readme-example`**: a plain `cargo test` never opens the README; the doctest is `no_run`. Nothing runs the code a reader copies, so none of the four rows above could fail a test |
| `cargo run --example mixer` (the panel) | `README.md:88-93` | `SIREN_FRAMES=90 SIREN_SHOT=… cargo run --example mixer` | **holds**: `siren: frames 90 mean_ms 16.829`, a 15 KB still, exit 0 |
| `cargo run --example music` | `README.md:95-100` | as written | **holds**: transitions at 0.01 s, 4.80 s, 7.20 s (2 and 3 bars of 2.4 s), `music: transitions on the bar: yes` |
| `cargo run --release --features mixer --example budget -- /tmp/budget` | `README.md:102-108` | in a debug build (same example; the disk has 3 GB) | **holds**: 40 asked, 40 played, 0 refused, peak 3 live voices, four files written |

## Ranked (`plan.md` has the table)

1. **The front door does not play a sound, shows nothing, and nothing runs it** — a reader who pastes the block gets silence and a warning about dead code; the one line on their console is Bevy's, and names neither this crate nor the key. Reproduced on a real window and device; four independent defects (never scheduled, keys without extensions, a `hear` that corrupts the log, a guide sentence that sends you to the wrong file) and one cause for all four surviving: **no test runs the block**.
2. A spatial sound that belongs to a moving entity is placed at it and its level follows it (119's carry-over: `.on(owner)` of a spatial cue with no `.at` is flat; the compensation is fixed at the play).
3. The layers of a score start together (3 of 100 warm-cache starts; design in `runs/2026-11-04-116/plan.md`).
4. A person listens.
5. The waiting items.

## After (`matrix-final.log`, the matrix on the final tree, run with `CARGO_INCREMENTAL=0`)

| command | before | after |
|---|---|---|
| `cargo test` | 9 lib + 226 + 6 doc | 9 lib + **233** + **7** doc |
| `--features serde` | 9 + 235 | 9 + **242** + 7 doc |
| `--features semantic` | 118 + 544 | 118 + **551** + 7 doc |
| `--features mixer` | 20 + 241 | 20 + **248** + 7 doc |
| `--all-features` | 129 + 572 | 129 + **579** + 7 doc |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 | **7** (the `SirenLog::since` example) / 1 |
| clippy `-D warnings`, `fmt --check`, `cargo doc -D warnings`, `cargo build --examples` (default and all features) | clean | clean |

+7 integration tests in every build (4 in `quick_start`, 3 cursor tests in `model`; none is feature-gated) and +1 doctest. The only warnings are the macOS linker message `ld: __eh_frame section too large` for a Bevy debug binary: `example "mixer"` always had it, and `example "quick_start"` — a `DefaultPlugins`
example — has it too; clippy, which does not link, is clean. **The cue table is 41 cases** (case 41 new), none red. `matrix-first-attempt-disk.log` is the first final matrix, whose two large links failed on a short disk (an inference, said where it is stated).

**The gate**: the render's 21 of 21 WAVs are byte-identical to iteration 119's (`cmp`), `mix.wav` is −21.5 LUFS, LRA 0.0, −7.5 dBFS and its spectrogram was read (`evidence/spectrograms/mix.png`); **the two new footsteps** were measured (−6.16 / −6.10 dBFS peak, −23.9 / −24.4 dBFS RMS,
−24.0 LUFS integrated for eight seconds of the walk) and their spectrograms, waveforms and attack and tail crops read (`evidence/spectrograms/step_0{1,2}-*.png`); `perfcheck` ×3 is inside its band and `push` and `since` were timed directly (`perf.md`); the block was **run on this Mac's window and output
device** (`evidence/after/quick-start-real-run.txt`); the console tests' captured lines are `evidence/after/quick-start-console-in-the-tests.txt`; a different asset directory, a three-variant cue and five seeds are `evidence/after/fresh-eyes-seeds-and-cue-set.txt`.

**Probes** (`evidence/before/`): the README block pasted verbatim, built and run for seven seconds in a window (a `never used` warning twice, no sound, one console line that is Bevy's); its `hear` (one play, two log entries, the second stamped `0.00s`); the `len()` cursor (256 of 600); the new tests' compile errors on the unchanged tree.
