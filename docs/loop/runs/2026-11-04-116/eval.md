# Iteration 116 — eval: the baseline, the cue table, and the claims audit

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

**Nothing is red.** Every command exits 0 and the counts are iteration 115's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + 5 lib + **203** |
| `--features serde` | 5 lib + **212** |
| `--features semantic` | **114** lib + **521** |
| `--features mixer` | **16** lib + **216** |
| `--all-features` | **125** lib + **547** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large (max 16MB)`* from `examples/mixer` against the full umbrella) is not this crate's code.

## The cue table (`cues.yaml`)

All **36** cases are asserted by the matrix (each case's `evidence:` names its tests) and none is red. **What the table cannot see**: no case concerns **what a play answers when its file did not load**. Cases 10 – 13 and 25 pin the refusals the crate can name — no cue, a key with no handle, no bus, no ears, bytes the decoder refuses — and case 25's own `source:` cites *"Bevy's own split of not-found from failed-to-load (bevy_asset `LoadState::Failed`)"*: the split was known and only one side of it was built. A file that *failed* is a state none of the cases builds, because every headless test's handle is a default (Uuid) one and the asset server answers `None` for it.

## Half-done work

None. `git status` is clean at `a6d12cb` (= `origin/main`); `runs/2026-11-03-115/` has its `review.md` (GREAT, GREAT, GREAT) and its log entry. Open issues, re-read through `issue://`: `#32`–`#34` the editor's (pinned 221 commits back), `#35` waits on a person's anchor pairs, `#39` was decided not to ship (ADR 0005). **No `consumer` issue is open.**

## The claims audit — by `grep` and `read`

Iteration 115's skill addition says to ask *"does any test loop a file of each format at all?"* and to read every sentence about **sound** against a test that reads one. This iteration asked the same of every sentence about **a voice and its file**:

| sentence | where | held by | verdict |
|---|---|---|---|
| *"Every play answers exactly one of `Played`/`Refused`. A refusal names what is missing; nothing plays on a refusal."* | `AGENTS.md` | `tests/play.rs` (no cue, no sounds, no handle, no bus, no room) — **no test with a file that failed** | **false for a failed load**: six plays of a typo'd file, six `Played`, no `Refused` (`evidence/before/`) |
| *a log of what played and why*; *"says why the music went quiet"* | `README.md` | the log's own tests | for a failed file the log says *played* and *stolen by ui.click (cue)* — a healthy sequence for a silent sound |
| `SirenSounds::failed` — *"every key whose load the server reported as failed"* | `docs/guide/cues.md:76-77` | one `sounds.rs` doc, no test | a pull API a game must remember to call; nothing in the crate's own answers uses it |
| *an unresolved handle is a voice that never gets a `MixerVoice` — … what the crate would see while a file is still loading* | `tests/mixer.rs:76-77` (a comment) | nothing | true, and it is also what the crate sees **for ever** when the load fails: no system ends such a voice (`retire_voices` needs a sink, a `Completes` or the mixer's `finished`) |
| *"it keeps every layer phase-aligned"* | `music.rs:6-8`, `docs/guide/music.md:20-22` | `a_silent_layer_stays_in_the_mix` (a voice exists) — **not alignment** | true only when every stem loaded before the score started; **measured: 3 of 100 starts of 8 real stems began in two frames** (`evidence/after/stem-skew.rs.txt`) — carved off |

## Ranked (`plan.md` has the full list)

1. **Bugs and wrong output — a play whose file failed to load is `Played`, forever silent, and the log calls it healthy.** Probed on the real `AssetServer`; deterministic; on the default cue; the commonest authoring mistake.
2. **The layers of a score start apart when their loads straddle a frame** — real and rare (3 of 100 here); carved off with a design.
3. Everything else in iteration 115's top five: a person listening (not mine); the Ogg-loop startup probe (rare configuration); the hubness measurement (experimental); the floor decision (the owner's); `play`'s per-play copy (performance).

## After (`matrix-final.log`, the third run of this tree)

The matrix ran three times on the final code and the log file is the last: the first (the `Files` struct) was **red on clippy alone** — *"type `sounds::Files<'_>` is more private than the item `play::play_requests`"*, quoted in `review.md` — the second (a type alias) was green, and the third (after the tests' sleeping loader became a gate) is the log here. The first two were overwritten by the same file name.

| command | baseline | after |
|---|---|---|
| `cargo build` | ok | ok, **no warning from the lib** (the first run carried `private_interfaces` on every line) |
| `cargo test` (the `all` binary) | 203 | **208** |
| `--features serde` | 212 | **217** |
| `--features semantic` | 521 (+114 lib) | **526** (+114) |
| `--features mixer` | 216 (+16 lib) | **222** (+16) |
| `--all-features` | 547 (+125 lib) | **553** (+125) |
| `cargo test --doc` / `readme::` | 6 / 1 | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean | clean |
| `cargo fmt --check` / `cargo doc` (`-D warnings`) | clean | clean |

**New: six tests** in `tests/failed_load.rs` (five without `mixer`): a play of a file that already failed; a voice whose load fails after it was played (refused in the very frame the state flips, and freed); the control — a load still in flight, held open for ten frames, is not refused, and what arrives plays; a failed *reload* whose bytes are loaded still plays; the crate's own mixer is never asked to play a failed load; a music layer with a dead stem. **No existing test was edited.** The only remaining warning is the `example "mixer"` linker one, which the baseline had.

**The cue table is 37 cases** (case 37 new), none red. **Render**: 21 of 21 WAVs byte-identical to iteration 115's, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix-spectrogram.png` read. `perfcheck` ×3 in `evidence/after/perfcheck.txt`; the system's own cost in `perf.md`.
