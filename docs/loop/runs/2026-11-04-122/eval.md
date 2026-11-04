# Iteration 122 — eval

## Baseline on arrival (`baseline-matrix.log`, `docs/loop/matrix.sh`, `CARGO_INCREMENTAL=0`, the tree at `3176ebe` = `origin/main`)

The tree was clean, `origin/main` was at `3176ebe`, and the open issues are `#32`–`#35` and `#39` — **no `consumer`
issue**. Free disk on arrival: **3.0 GB** (the machine's volume is at 100 %).

**The matrix script moved to `docs/loop/matrix.sh`** and has **one** change from iteration 112's: `cargo test
--all-features` is `cargo test --all-features --tests -j 1` there, because the single command links the lib's tests,
`all` and seven Bevy example binaries in parallel and is the step iteration 121 could not fit on disk. The examples are
still built, with the same features, by `cargo build --examples --all-features`.

| step | result |
|---|---|
| `cargo build` | green |
| `cargo test` | **9 + 241 + 7 doc** green |
| `cargo test --features serde` | **9 + 250** green |
| `cargo test --features semantic` | **RED — did not build**: `rustc-LLVM ERROR: IO failure on output stream: No space left on device` at **2.1 GB free**. **Not the code**: re-run alone at 2.2 GB it passed, **118 + 559** |
| `cargo test --features mixer` | **20 + 256** green |
| `cargo test --all-features --tests -j 1` | **129 + 587** green |
| `cargo test --doc` | **7** green |
| `cargo test --features readme-example --test all readme::` | **1** green |
| `cargo build --examples` / `--all-features` | green (the second one, green at **2.8 GB**, is the step iteration 121 could not link) |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `cargo doc --no-deps --all-features` | green |

## The cue table (`docs/loop/cues.yaml`)

42 cases on arrival, **none red**. Case **3** — *a burst repeats one sample and does not vary its pitch* — is the cue
this iteration is about. Case **43** is new (see below).

## The claims audit: what the stream promises, held against a play that cannot use its number

`docs/guide/cues.md` said the coupling out loud (*a project's soundscape is a function of its seed and its play order*).
The audit asked it through the real play path: a three-variant `Shuffle` observer, six plays, after 0 / 1 / 8 plays of a
cue that cannot decide.

**Before** (`evidence/before/draws-on-the-unchanged-tree.txt`): the observer alone is `b b c a c b`; after eight plays of
a **one-sound exact-pitch** cue (a burst, a UI click — **no decision in any play**) it is `c b a b a a`; after eight
alternating footsteps `a a b c b c`; an alternating **exact-pitch** footstep has **one** decision in eight plays and still
moves it, and one play and eight plays differ.

**After** (`evidence/after/draws-after.txt`): the burst and the click leave it **`b b c a c b`** — exactly where the seed
put it; the exact-pitch footstep's **one play and eight plays** leave it in the same place (`a c a c b c`); the default-pitch
footstep still moves it (a detune is a real decision), and a three-variant `avoid: 1` cue still draws every play and
still never repeats back to back.

## Ranked (`plan.md` has the table)

1. **A draw that cannot change the answer is spent anyway** — chosen. It is wrong output on the guide's own sentence,
   measured, and its release-note cost was already paid by iteration 121's unreleased stream change.
2. **Authoring refusals are silent on a console nobody reads the log of** (119's and 120's carry-over).
3. **A spatial sound on a moving owner** is flat and fixed at the play (119's).
4. **The layers of a score start together** (116's design).
5. **The baseline does not fit the disk when the dependency artifacts are warm** — a blocker class; `docs/loop/matrix.sh`
   and `AGENTS.md` now say what that error means, and the footprint is recorded in `perf.md`.

## After

**The matrix** (`matrix-final.log`): **9 + 243 / 9 + 252 / 118 + 561 / 20 + 258 / 129 + 589**, 7 doc, 1 readme, examples
both ways, clippy `-D warnings`, fmt, rustdoc `-D warnings` — **+2 integration tests in every build** (two in
`tests/model.rs`; the third new test is the rewritten `tests/play.rs::an_exact_pitch_range_leaves_the_stream_alone`,
which replaces `an_exact_pitch_range_still_draws_from_the_stream`). **Cue table**: 43 cases, none red. **The render
set**: 21 of 21 WAVs and every refusal text byte-identical to iteration 121's; `render.txt` differs in **one** `variants`
line; `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS, its spectrogram read (no click, no hole, no shelf). **`perfcheck`**:
0.047 – 0.057 / 0.283 – 0.325 / 0.846 – 1.115 µs, inside iteration 121's band. **The public surface**: unchanged (no new
field, no new message; `VariantPicker`'s behaviour on a forced pick is the one semantic change, and it is a release note).
