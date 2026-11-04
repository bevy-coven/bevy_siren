# Iteration 119 — eval: the baseline, the cue table, and the claims audit

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

Tree clean at `a71a264` (= `origin/main`); open issues `#32`–`#35` and `#39` (re-read through `issue://`), **none labelled `consumer`**. **Nothing is red.** Every command exits 0 and the counts are iteration 118's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + 9 lib + **217** |
| `--features serde` | 9 lib + **226** |
| `--features semantic` | **118** lib + **535** |
| `--features mixer` | **20** lib + **232** |
| `--all-features` | **129** lib + **563** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large`* from `examples/mixer` against the full umbrella) is not this crate's code; the baselines of iterations 115 – 118 carried it too. Disk: 4 GB free on arrival; the matrix's guard cleaned this crate's artifacts twice and it finished with 6 GB.

## The cue table (`cues.yaml`)

All **39** cases are asserted by the matrix (each case's `evidence:` names its tests) and none is red. **What the table cannot see**: **one** case concerns an owner (32: *a sound whose owner is despawned stops, and the log says which sound and why*) and it builds the happy path — an owner spawned and flushed frames earlier, with no `.at(..)` — and then despawns it. No case builds an owner that is **not yet spawned**, one that is **already gone**, one with **no `Transform`**, or a play with **both** `on` and `at`, which is the one `Play`'s own doc example writes. **Case 40 is new** (`cues.yaml`).

## Half-done work

None. `git status` was clean at `a71a264` (= `origin/main`); `runs/2026-11-04-118/` has its `review.md` (GREAT, GREAT, GREAT) and its log entry.

## The claims audit — by running the documented use

Iteration 118's lesson was *"run the documented use of a handle the crate hands out, across the scenarios that end the thing it names."* This iteration ran the **other direction**: the documented ways a game *hands the crate* an entity — `Play::on` — across the states that entity can be in. `examples/scratch_owner.rs` (release, headless, Bevy's path, `TransformPlugin`; kept as `evidence/after/owner-probe.rs.txt`) built each; its output on the unchanged tree is `evidence/before/owner-probe-on-the-unchanged-tree.txt`.

| claim | where | state run | result |
|---|---|---|---|
| *"The sound belongs to `entity`"*; a footstep's foot, an engine's vehicle | `Play::on`'s doc | the owner **allocated and not yet spawned** (a `Commands::spawn` in the system that writes the play, unordered against the crate's; and `world.entity_allocator().alloc()` for a deterministic one) | **a panic** in Bevy's command handler (`handler.rs:128`, *"Entity not yet spawned"*), naming no system. With the game system ordered `before(Choose)`, in `PreUpdate`, or in `PostUpdate`, it works — **the crash depends on an arrangement a game does not control** |
| *"The sound is at `position` in the world"*; the example `.on(foot).at(Vec3::new(2.0, 0.0, 1.0))` | `Play::at`'s doc, `Play`'s doc | an owner **at** the world position asked for | the voice is at **(200, 0, 0)**, not (100, 0, 0): `ChildOf` composes the owner's transform with `at`. Level 12.04 dB under the crate's law |
| the same | | an owner with **no `Transform`** | the voice stays at **(0, 0, 0)** — the listener — because Bevy never propagates a child of a Transform-less parent. The crate writes the sink **+52.04 dB** (its +80.00 dB panner compensation stands where the panner does nothing): 400× the sample |
| *"a sound `Play::on` hung from an entity stops when that entity is despawned, and says so"* | `docs/guide/buses.md:112` | an owner **already gone** at the play | no `ChildOf` (Bevy warns and removes it), a loop that lives **for ever**, counted on its bus, the log saying only *played* |
| *"A game that despawns a voice it never parented gets no line"* | `docs/guide/buses.md:114`, `mix.rs` `log_despawned_voices` | the **game despawns the voice** while its owner lives | the log says `stopped … owner despawned`: **false** — `ChildOf` is read as *"the owner did it"* |
| *an owner despawns → the voice goes, one line* | cue case 32; `tests/play.rs`, `tests/mixer.rs` ×2 | an owner spawned and flushed frames earlier | **holds** (the control) |

## Ranked (`plan.md` has the table)

1. **`Play::on` hangs a voice from its owner by making it a child — a crash, a wrong level and pan (up to +52 dB over the sample), an ownerless loop and a false log line from one line of code.** Reproduced; deterministic (`pending`); one mechanism fixes all four; no new `Refusal` variant.
2. The README's quick start does not play a sound as written (iteration 118's carry-over; fails loudly since 116).
3. The layers of a score start together (3 of 100 warm-cache starts; design in `runs/2026-11-04-116/plan.md`).
4. A person listens.
5. The waiting items.

## After (`matrix-final.log`, the matrix run on the final tree, with the scratch example already removed)

| command | before | after |
|---|---|---|
| `cargo test` | 6 doc + 9 lib + 217 | 6 doc + 9 lib + **226** |
| `--features serde` | 9 lib + 226 | 9 lib + **235** |
| `--features semantic` | 118 lib + 535 | 118 lib + **544** |
| `--features mixer` | 20 lib + 232 | 20 lib + **241** |
| `--all-features` | 129 lib + 563 | 129 lib + **572** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 | 6 / 1 |
| clippy, `fmt --check`, `cargo doc -D warnings`, `cargo build --examples` (default and all features) | clean | clean |

+9 integration tests in every build (`tests/owner.rs`; none is feature-gated: the mixer one uses a fake output). The one warning is the known `example "mixer"` linker one. `matrix-final-first.log` is the matrix before the ninth test was added; `matrix-final.log` is the last. **The cue table is 40 cases** (case 40 new), none red.
**Render**: 21 of 21 WAVs byte-identical to iteration 118's (`cmp`), `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, `evidence/spectrograms/mix.png` read. `perfcheck` ×3 (`evidence/after/perfcheck.txt`) and the new system's cost, timed directly (`evidence/after/owner-system-cost.txt`), are in `perf.md`.

**Probes** (`evidence/`): the same eight modes on the unchanged and the final tree — the panic for an owner allocated and not spawned (an unordered system and `world.entity_allocator().alloc()`) gone; the voice at (100, 0, 0) for an owner at the place and for one with no `Transform`, where it was (200, 0, 0) and (0, 0, 0); a loop on a gone owner ended in its frame; no false `owner despawned` line.
