# Iteration 118 — eval: the baseline, the cue table, and the claims audit

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

Tree clean at `84ac075` (= `origin/main`); open issues `#32`–`#35` and `#39` (re-read through `issue://`), **none labelled `consumer`**. **Nothing is red.** Every command exits 0 and the counts are iteration 117's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + 9 lib + **213** |
| `--features serde` | 9 lib + **222** |
| `--features semantic` | **118** lib + **531** |
| `--features mixer` | **20** lib + **227** |
| `--all-features` | **129** lib + **558** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large`* from `examples/mixer` against the full umbrella) is not this crate's code; the baselines of iterations 115 – 117 carried it too.

## The cue table (`cues.yaml`)

All **38** cases are asserted by the matrix (each case's `evidence:` names its tests) and none is red. **What the table cannot see**: no case concerns **what a game does with the `Entity` a `Played` hands it** — the use that field's own doc names — on either backend, nor whether the README's quick start plays a sound. Cases 10 – 13, 25, 32, 37 and 38 pin the refusals and the owner's despawn the crate can name; every one reads messages and counts voices, and none runs the one line a game would write. **Case 39 is new** (`cues.yaml`).

## Half-done work

None. `git status` was clean at `84ac075` (= `origin/main`); `runs/2026-11-04-117/` has its `review.md` (GREAT, GREAT, GREAT) and its log entry.

## The claims audit — by `read` and by running

Iteration 117's lesson was *"read the `unwrap`s of the code your entity is handed to"*. This iteration asked the same of the **front door**: *which sentence a new reader runs first, and which documented use of a returned handle, has any test run it as a game would?* `examples/scratch_front_door.rs` (release, a real device, real files; kept as `evidence/after/front-door.rs.txt`) ran each; its output on the unchanged tree is `evidence/before/front-door-probe-on-the-unchanged-tree.txt`.

| claim | where | how it was checked | result |
|---|---|---|---|
| *"Both answer `Played`, `Refused` and `Voice` identically"* | `README.md:83`, `src/lib.rs:65`, `AGENTS.md:97` | `tag-refused`: the crate's mixer behind an output whose `play` answers `None` (what `rodio` does for bytes it refuses) | **false for bytes the decoder refuses**: `Played` and `Refused` in **one frame**, the entity despawned in it (Bevy's path refuses at the play since 117) |
| *`Played::entity`: "for a game that hangs its own data on it"* | `src/play.rs:93` | a system after `SirenSystems::Play` doing `commands.entity(p.entity).insert(Tag)` | **panics** in `bevy_ecs-0.20.1/src/error/handler.rs:128` (*"Entity despawned"*) on the path above — naming the game's system, not the file |
| the quick start | `README.md`, `src/lib.rs`, `tests/readme.rs` | `quick as-written` and `quick with-extensions`: the snippet's `setup` against real `.ogg` files through the real `AudioPlugin` | **does not play as written**: keys `step_01`/`step_02` and `load_table(.., "audio")` ask for `audio/step_01` (no extension), so every play is refused (the log says why since 116); with `.ogg` in the keys 5 of 5 `Played`; `walk` and `hear` are never scheduled; `hear` re-pushes what the plugin already logs; `tests/readme.rs` holds only that the README equals the doctest's text (a `no_run`) |
| a `Play::on(owner)` for an owner that is already gone | `Play::on`'s doc | `owner-bevy`, `owner-mixer` | `Played`, a voice **without** a `ChildOf` (Bevy warns and removes the relationship: `bevy_ecs-0.20.1/src/relationship/mod.rs:207`), a `Mode::Loop` voice that lives **for ever** on both backends, counted on its bus, the log saying only *played* (the control with a live owner logs `stopped … owner despawned`) |
| a key `step_03` is the file `audio/step_03.wav` | `docs/guide/cues.md:147-149` | the same read of `load_table` | **contradicts** the quick start's fix: a key is used verbatim as the path |

Found **while testing the fix**, by the invariant test's first form (*a `Played` entity exists when the frame ends*): a voice stolen by a later request **in the frame it was announced**, or stopped by a `StopCue` in it, is despawned in that frame (`spawn_voice` gives it `Applied { db: SILENT_DB }`, so `apply_stopping` finds it already silent). It never sounded and the log says `stopped … stolen by …`; the invariant was narrowed to what the crate can honestly promise and the exception documented on `Played::entity` (`plan.md`, `review.md`).

## Ranked (`plan.md` has the table)

1. **On the crate's own mixer, `Played` then `Refused` in one frame, the entity despawned — and a game that uses `Played::entity` as documented panics.** Reproduced; deterministic; falsifies three sentences; the fix is small and general.
2. The quick start does not play a sound as written. A loud failure since 116 (every play refused *with the cause*), so below a crash; first in the next pass.
3. `Play::on` for a vanished owner: an ownerless loop for ever, a leaked budget slot. A silent bug, but it needs an owner despawned before the play, and the safe design is larger than a check (an existence test at play time is **unsafe**: an entity spawned in the same frame through deferred commands looks gone, which would refuse valid plays).
4. The layers of a score starting apart (iteration 116's carve-off; 3 of 100 warm-cache starts).
5. A person listens (the standing owed item).

## After (`matrix-final.log`, the matrix run on the final tree, with the scratch example already removed)

| command | before | after |
|---|---|---|
| `cargo test` | 6 doc + 9 lib + 213 | 6 doc + 9 lib + **217** |
| `--features serde` | 9 lib + 222 | 9 lib + **226** |
| `--features semantic` | 118 lib + 531 | 118 lib + **535** |
| `--features mixer` | 20 lib + 227 | 20 lib + **232** |
| `--all-features` | 129 lib + 558 | 129 lib + **563** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 | 6 / 1 |
| clippy, `fmt --check`, `cargo doc -D warnings`, `cargo build --examples` (default and all features) | clean | clean |

+4 integration tests in every build (`tests/one_answer.rs`) and one more with `mixer` (`tests/mixer.rs::the_real_mixers_decode_error_agrees_with_its_play`, which returns early where there is no audio device; it ran here, on this Mac's). The one warning is the known `example "mixer"` linker one. **The cue table is 39 cases** (case 39 new), none red.
**Render**: 21 of 21 WAVs byte-identical to iteration 117's (`cmp`), `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix.png` read. `perfcheck` ×3 and the added work, timed directly, are in `perf.md`.

**Probes** (`evidence/`): the crate's own mixer on a real device with fourteen real files and a game system tagging every `Played::entity` — 8 `Played`, 6 `Refused` at the play, 140 frames, no panic; the original probe with an output that says beforehand (survives) and one that does not (still panics: the documented limitation); Bevy's decoder builder against the mixer's `Decoder::new` on the same fourteen (14 of 14 agree); the check's first-sight cost per format.
