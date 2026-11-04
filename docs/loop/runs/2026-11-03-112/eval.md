# Iteration 112 — eval

## Baseline before anything (2026-11-03 21:12, commit `7ddf91f`, a clean tree)

**Everything the skill runs was green.** The numbers are iteration 111's "after", unchanged:

| run | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 199 + 6 doc |
| `cargo test --features serde` | 208 + 6 |
| `cargo test --features mixer` | 209 + 6 |
| `cargo test --features semantic` | 109 lib + 497 + 6 |
| `cargo test --all-features` | 109 lib + 521 + 6 |
| `cargo test --doc`; `--features readme-example --test all readme::` | 6; 1 |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |
| `cargo build --examples` (default, and `--features semantic`) | ok |

The 33 cases of `cues.yaml` are asserted by that matrix (`tests/play.rs`, `mix.rs`, `music.rs`, `table.rs`, …) and are unchanged; the 21
rendered WAVs are compared again at the end.

## What was red, or odd

1. **The disk, again.** 12 GB free at the start; the matrix itself took it to **951 MB** (the crate's debug artifacts are 11.5 GB for five
   feature sets). My guard that was meant to clean *this crate* below 5 GB **did not run**: it called `df -g`, which this machine's `df`
   rejects (`error: unexpected argument '-g'`), and the error was swallowed by the pipeline. Nothing was lost — the matrix finished before
   the disk did — but the guard was decoration. `df -k ~ | awk 'NR==2{print int($4/1048576)}'` is the form that works here.
   `cargo clean -p bevy_siren --target-dir target`: dry run **861 files, 15.6 GiB**; the oldest artifact is from 17:08 today
   (iteration 110's release build), every one built by this loop; **786 files, 11.5 GiB removed** → 11 GB free.
   `~/.cache/huggingface` is the owner's and was not touched.
2. **Two run directories have no `review.md`**: `2026-11-03-78` and `2026-11-03-98`. `-78` is iteration 78's research-only ADR 0005; its
   verdicts (*pass 1 GREAT, pass 2 GREAT, pass 3 GREAT*) are in `log.md` and the file was never written. `-98` is not an iteration's
   directory at all: it holds one file, `eval-baseline.txt`, which `examples/evalbench` writes (the path is hard-coded) and the log has no
   *Iteration 98* heading. Neither is half-done work; both are noted so the next orient step does not re-flag them.
3. **The editor pins this crate 215 commits back.** `bevy_grimoire/crates/bevy_cauldron/Cargo.toml` takes `bevy_siren` at git rev
   **`82d227e`** (2026-11-02 23:39, *"cue: problems_in reads the buses the mix actually has"*) with `features = ["serde"]`. The editor's
   audio-panel test therefore runs against that rev, not against `HEAD`, whatever this loop does — which is why it has not been re-run
   for three iterations and why that has cost nothing: a pin cannot be broken from here. The editor checkout is `ahead 2` of its remote
   with unpushed audio commits (`d6bb032`, `799fad2`), the editor's own work, left alone. Moving the pin is the editor's commit in its
   own repository; it is recorded here because 215 commits is a long way to move at once.

## Open issues (re-read in full)

`#32`–`#34` are the editor (`bevy_grimoire`; none is filed there with `consumer:bevy_siren`); `#35` is open on criterion 3 (a person's
anchor judgements); `#39` is decided not to ship (ADR 0005) and left open for the owner. Nothing is filed as a `consumer` request.

## After (the same matrix, `matrix.sh` / `matrix.log`; every command exits 0)

| | before (`7ddf91f`) | after | why it moved |
|---|---|---|---|
| `cargo test` | 199 + 6 doc | **200** + 6 | the header test is not feature-gated |
| `--features serde` | 208 | **209** | the same |
| `--features semantic` | 109 lib + 497 | 109 + **505** | +7: FLAC, MP3, Ogg decode, the Ogg end trim, a chained Ogg, every-extension, a folder of every format |
| `--features mixer` | 209 | **210** | the header test |
| `--all-features` | 109 + 521 | 109 + **529** | +8 |
| doc tests / readme / examples both ways / clippy `-D warnings` / fmt / rustdoc `-D warnings` | green | green | — |

`cargo tree --edges normal`: default **413** lines, 149 crates (unchanged); `--features semantic` **487 → 497**, 181 → **183** crates; `--features mixer` 450 → 460
(`evidence/after/dependency-graph.txt`). **The render:** all 21 WAVs byte-identical to iteration 111's; master −21.5 LUFS, −7.5 dBFS true peak. **Perf:** `perf.md`.

One warning appears on every `cargo test` and `cargo build --examples`: *"linker stderr: ld: __eh_frame section too large (max 16MB) to encode dwarf unwind offsets"*
from linking `examples/mixer` against the full umbrella in a debug build. It is not from this iteration (`examples/mixer.rs` is unchanged since `85f4975`) and is
not a rustc warning; clippy and rustdoc are `-D warnings`-clean.

**The disk guard worked this time** — the guard in `matrix.sh` uses `df -k` — and cleaned this crate's artifacts twice (after `--features semantic` and `--all-features`);
free space never fell below 4.5 GB.
