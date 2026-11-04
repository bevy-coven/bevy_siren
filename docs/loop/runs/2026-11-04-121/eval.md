# Iteration 121 — eval: the baseline, the cue table, and the claims audit of what a seed promises

## Baseline on arrival (`baseline-matrix.log`, the matrix script from iteration 112, `CARGO_INCREMENTAL=0`, the tree at `ed1caff` = `origin/main`)

The tree was clean and `origin/main` was at `ed1caff`; the open issues are `#32`–`#35` and `#39`, none labelled `consumer`.

| command | result |
|---|---|
| `cargo build` | green |
| `cargo test` | 9 lib + **233** + 7 doc, green |
| `--features serde` | 9 + **242** |
| `--features semantic` | 118 + **551** |
| `--features mixer` | 20 + **248** |
| `--all-features` | 129 + **579** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 7 / 1 |
| `cargo build --examples` | green (the two known linker messages: `example "mixer"` and `example "quick_start"`) |
| **`cargo build --examples --all-features`** | **red at 3.6 GB free: `linking with cc failed` on `example "mixer"`** — and **green, `Finished in 14.37s`, at 4 GB free** |
| `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

**The one red is the disk, not the code** (an inference: the script keeps no line of `ld`'s message; the evidence is that the same command passes with 4 GB and fails with 3.6). Building every example with every feature now takes **about 3 GB** of this crate's debug binaries (iteration 120's `quick_start`, a
`DefaultPlugins` example, is 500 MB of it). `cargo clean -p bevy_siren` cannot see the binaries of a *deleted* example, so ~2.8 GB of this loop's own scratch-example binaries (`scratch_*` in `target/debug` and `target/release`; their sources are long deleted and each is regenerable from its evidence copy) were removed by hand. **`~/.cache/huggingface` holds 50 GB and is the
owner's; it was not touched.**

## The cue table (`docs/loop/cues.yaml`)

41 cases, **none red** (the suite that holds them is the matrix above). Case 17 — *"the same seed replays the same sounds, gains and pitches; two seeds are two streams"*, observed with the seeds **7 and 99** — is green, and **it is the case this iteration's audit shows to be true only for large-looking seeds**.

## The claims audit: what a seed promises, held against `Rng`

Iteration 120's fresh-eyes pass saw five seeds open on the same recording and measured sixty; this asks the claim itself, over every small seed, of the one place a seed becomes a stream (`model::Rng`, which is all `SirenChoices` and `examples/render` use).

| claim | where | what I ran | result |
|---|---|---|---|
| *"Two projects that differ only in `seed` play different variants and different detunes from the first play"* | `docs/guide/cues.md:211-214` | the first pick of a cue with k variants for seeds 1 … 3000 (`floor(first draw × k)`, what `VariantPicker::pick_from` does on a fresh picker), k = 2, 3, 4, 8 (`evidence/before/seed-probe-on-the-unchanged-tree.txt`) | **false for every one of them: 3000 of 3000 pick the first variant, at every k** (χ² = 3000 × (k − 1), p = 0). The first draw of every seed up to 4096 is in the lowest sixteenth of [0, 1); `Rng::new(1).next_u32()` is exactly `0x00000000` |
| *"two seeds are two streams from the first play"* | `README.md:133` | Pearson r between the first draws of seed s and s + 1, seeds 1 … 3000 | **r = 0.9995** (independence: within ±0.055); and draw *n* of neighbouring seeds stays correlated at +0.65 (n = 2), −0.47, −0.60, **−0.78** (n = 6), −0.46, −0.28, +0.50 (n = 24), −0.43, +0.40 (n = 64) |
| *"a capture, a replay and an A/B of an audio change are reproducible"* | `README.md:132`, `docs/guide/cues.md:214` | flip one bit of the seed and count the bits of the first draw that change (a good mixer: 16 of 32) | **3.8 of 32**; seed bits 0 and 1 change **none** of them, bits 2 – 5 change one. An A/B on seeds 1 and 2 is the same footstep |
| (nothing says so; a *linear* generator must not be seeded linearly) | Brent 2007 §4 | for seeds a, b and c = a ⊕ b, is draw *n* of c equal to draw *n* of a ⊕ draw *n* of b? | **yes, for all of the first 1000 draws, in 1953 of 1953 triples** (a < b ≤ 63): seeds 1, 2 and 3 are not three streams — the third is the first xor the second |
| *"the same number plays the same sequence twice, field for field, in two processes"* | `docs/guide/cues.md:213` | the existing tests (`the_rng_repeats_its_own_sequence`, `a_projects_seed_reaches_the_stream`, `changing_the_seed_restarts_the_stream`) | **holds** (and the new stream must keep it) |
| the default seed and a seed of `0` are one stream; `seed()` reports what the stream was built from | `docs/guide/cues.md:244-247`, `model.rs` | `tests/model.rs` | **holds** (and must keep holding: a public contract) |

Why nothing noticed: the crate's own tests assert *properties* with seeds 3, 5, 7, 11 and 99 (equal within a seed, different across seeds — and "different across seeds" holds for the whole stream, just not for its opening), and the default seed, `0x5EED51AE00000001`, has 20 of 64 bits set in two clusters and its first draws look unremarkable (`b42b988d 58628b1d 490ed9cc e9e82178`).
`examples/render` does not see it either: it renders each cue at `pitch.at(0.0)` and uses the stream only for its `variants …` report lines.

## Ranked (`plan.md` has the table)

1. **A seed is not seeded: `Rng::new(1)` starts a stream whose first draw is zero, so every small seed opens on the first variant, neighbouring seeds are correlated for dozens of draws, and related seeds are exactly xor-related streams.** Wrong output on a documented guarantee, reproduced over 3000 seeds; the fix is one nonlinear scramble in `Rng::new`; **it changes every seeded stream** (`needs-decision/exact-pitch-draw.md` is the precedent for why that is a release note and not a patch).
2. Authoring refusals are silent on a console nobody reads the log of (119's and 120's carry-over).
3. A spatial sound that belongs to a moving entity is placed at it and its level follows it.
4. The layers of a score start together.
5. A person listens, and the waiting items.

## After

**The matrix** (`matrix-final.log`, the final tree, `CARGO_INCREMENTAL=0`): **9 + 241 / 9 + 250 / 118 + 559 / 20 + 256 / 129 + 587**, 7 doc, 1 readme, `cargo build --examples` both ways, clippy `-D warnings`, fmt, rustdoc `-D warnings` — **+8 integration tests in every build** (seven in `tests/model.rs`, one in `tests/play.rs`), no lib or doc test added. **One step did not link: `cargo test --all-features`, at 3.9 GB free** — **[INFERENCE] the disk and not the code**: run again by hand it started at 2.9 GB and ended at 125 MB free with the same error; after this crate's own artifacts were cleaned, `cargo test --all-features --tests -j 1` linked and ran **129 + 587 passed**, and the examples are built by the matrix's own `--examples --all-features` step, which passed (the baseline failed that step on the same disk, at 3.6 GB, and passed at 4.0 GB). The mixer step reads **1.88 s**; a first form of the play-level test made it 13.21 s (`evidence/before/matrix-with-120-apps.txt`).

**The claims audit, run again on the changed tree** (`evidence/after/seed-probe-after.txt`; the program is `evidence/after/seed-probe.rs.txt`): the first pick of a cue of 2, 3, 4 or 8 variants over the seeds 1 … 3000 is spread evenly (χ² 0.09 – 1.32, p 0.52 – 0.99; before: 3000 of 3000 on the first variant); the first draw in sixteenths χ² 9.2 (p 0.87); neighbouring seeds' first draws r = −0.0055 (before 0.9995) and \|r\| ≤ 0.053 at every depth to 64; one flipped seed bit changes 16.007 of 32 bits (before 3.783; the worst bit, 0, 15.45); **0 of 1953** xor triples (before 1953). **The cue table**: 42 cases, none red; case **42** is new (*a seed opens a stream the way a random seed would*). **The render set**: 21 of 21 WAVs byte-identical to iteration 120's, `render.txt` differs in three `variants …` lines; `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS and its spectrogram read. **On the real device**: the quick start's first step on the default seed was `step_02.ogg` and is `step_01.ogg` (`evidence/after/quick-start-first-step.txt`).
