# Iteration 110 — eval

## Baseline before anything

**The disk was the first red.** `df -h ~` read **4.6 GB free (100 %)**, which the skill ranks above every feature: a full matrix
rebuild wants ~9 GB. `target/` was **30 GB** (`debug/deps` 14 GB, `debug/examples` 12 GB, `release` 4.1 GB).

What I did, and why it was within what I may delete:

- `cargo clean -p bevy_siren --target-dir target --dry-run` first: **1 392 files, 18.4 GiB, this crate's own artifacts only** —
  no dependency.
- **Provenance check:** `find … ! -newermt '2026-11-03 00:00'` over `all-*`, `bevy_siren-*` and `libbevy_siren-*` found **0** files
  older than today; the oldest `all-*` is **16:40 today**, after the skill's own earlier clean (*"measured 2026-11-03"*). Every
  artifact is a product of this loop's builds, and the skill names this exact command for this exact condition.
- `cargo clean -p bevy_siren --target-dir target`: **1 329 files, 15.0 GiB removed**, free space 4.6 → **18 GB**. `~/.cache/huggingface`
  (50 GB) is the owner's and was not touched.

The rebuild of the whole matrix then took **8.7 GB back** (18 → 9.3 GB): five feature sets each link their own ~1 GB test binary.
That is why this iteration verifies with targeted tests and runs the full matrix once, at the end.

After the clean, nothing was red:

| run | result |
|---|---|
| `cargo test` | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 doc |
| `cargo test --features mixer` | 209 + 6 doc |
| `cargo test --features semantic` | 109 lib + **479** + 6 doc |
| `cargo test --all-features` | 109 lib + **503** + 6 doc |
| clippy `-D warnings`, fmt, rustdoc `-D warnings` | clean |
| `cargo build --examples` (default, and `--features semantic`) | ok |

## The cue cases

All 33 of `cues.yaml` green, unchanged: this iteration's item is an example and no code on the mix, music or play path.

## After

| run | before | after |
|---|---|---|
| `cargo test` | 199 + 6 doc | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 | 208 + 6 |
| `cargo test --features mixer` | 209 + 6 | 209 + 6 |
| `cargo test --features semantic` | 109 lib + 479 + 6 | 109 lib + **482** + 6 |
| `cargo test --all-features` | 109 lib + 503 + 6 | 109 lib + **506** + 6 |
| `cargo test --doc`, `--features readme-example --test all readme::` | 6, 1 | 6, 1 |
| clippy `--all-targets --all-features -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean | clean |
| `cargo build --examples` (default, and `--features semantic`) | ok | ok |
| `render`: WAVs against iteration 109's | — | **21 of 21 byte-identical**; master −21.5 LUFS, −7.5 dBFS TP; spectrogram read (`evidence/spectrograms/mix.png`) |

Two failures on the way, both mine and both found by the run: **clippy `never_loop`** on a `for … { …; return; }` over refusals in the
example (rewritten as `if let Some(..) = refused.read().next()`), and a **rustfmt** diff on a long `println!`. The three new tests are
`a_stamp_names_the_export_that_made_the_vectors`, `an_index_from_another_export_is_served_once_its_version_is_given` and
`a_bad_k_is_explained_by_its_own_case` (this last one **run against the old message and failed** at `tests/semantic_tools.rs:530`,
then passed on the fix); `the_binary_says_what_it_takes_when_misused` gained a row.

**Not run: the Cauldron's panel test** (`cargo test -p bevy_cauldron … audio_panel_reads_the_crates_file` in `~/bevy_grimoire`). The
skill asks for it with a 14 GB `CARGO_TARGET_DIR`; this disk had **4.6 GB free** after the matrix. Nothing the panel reads changed —
`CueTable`, `SirenSettings` and `MusicScore` and their RON are untouched, and the only public addition is `ModelStamp::clap`.

## The real-model runs (`evidence/after/`)

| proof | file | result |
|---|---|---|
| no model → says what is missing | `refusal-no-model.txt` | exit **2**, lists `--model`, `--vocab`, `--audio-model`; **nothing written** |
| **before:** the first real run | `../before/first-real-run-batch3-graph.txt` | **0 indexed, 14 failed** (`Reshape_2`, `{0,96,64,3,21}`); an empty index was written |
| **after:** the same folder | `first-real-run.txt` | **14 indexed in 0.8 s**; index written; ranking printed |
| the crate against `transformers` | `parity.txt` | 5 phrases × 14 files: max difference **4.89 × 10⁻⁴**, same ranking order every time |
| a played run | `play-real.txt` | `Played audition.89f4c994c2850cba on the Music bus at +0.0 dB, variant Sosumi.wav`, then the voice ended and `done` |
| `siren-tools` with the real text graph | `siren-tools-real-model.txt` | start-up 0.9 s; `search_music` 4.6–5.6 ms; the first hit's score **0.20648861** is the example's |
| indexing throughput | `throughput-280.txt` | 280 clips in 16.9 s = **16.6 clips/s** |

The played run is evidence the crate answered `Played` and the voice ended by itself; it is **not** evidence a person heard it —
nothing here listens. With no output device the example prints *"(no audio device opened …)"* after 0.6 s, and it did not.
