# Iteration 123 — eval

## Baseline on arrival (`baseline-matrix.log`, `docs/loop/matrix.sh`, the tree at `bdc0bdf` = `origin/main`)

The tree was clean, `origin/main` was at `bdc0bdf`, and the open issues are `#32`–`#35` and `#39` — **no `consumer`
issue**. Free disk on arrival: **2.7 GB**, on a volume at 100 %.

| step | result |
|---|---|
| `cargo build` | green |
| `cargo test` | **9 + 243 + 7 doc** green |
| `cargo test --features serde` | **9 + 252** green |
| `cargo test --features semantic` | **RED — did not link** (`linking with cc failed`) beside the other feature sets' artifacts at 2.7 GB free. **Not the code**: re-run alone, **561 passed** |
| `cargo test --features mixer` | **20 + 258** green |
| `cargo test --all-features --tests -j 1` | **129 + 589** green |
| `cargo test --doc` / `readme::` | **7** / **1** green |
| `cargo build --examples` | green |
| `cargo build --examples --all-features` | **RED — did not link** (the same disk; `cargo check --examples --all-features` compiles every example) |
| `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt --check`, `cargo doc` | green |

## The cue table (`docs/loop/cues.yaml`)

43 cases on arrival, **none red**. Case **44** is new (below). The cases this iteration is about are the refusals — 10,
11, 12, 18 (`UnknownCue`, `MissingSound`, `UnknownBus`, `problems()`) and the two that must **not** be treated as faults,
7–9 and 21 (`VoiceLimit`).

## The claims audit: *"There is no silent failure"*

`docs/guide/cues.md` says it, `src/log.rs` says the opposite about printing, and the two had never been read together. A
probe (`examples/scratch_refusals.rs`, deleted; source in `evidence/{before,after}/refusals-probe.rs.txt`) plays every
refusal class through the real plugin and counts the events whose target is `bevy_siren`, with a `tracing` layer:

| class | plays | console **before** | console **after** |
|---|---|---|---|
| a healthy footstep, two handles | 8 | **0** | 0 |
| `footstep.grss` — not in the table | 1 | **0** | **1** |
| the same, 100 plays in one frame | 100 | **0** | **1** |
| two unknown cues in one frame | 2 | **0** | **2** |
| the same unknown cue over 8 frames | 8 | **0** | **8** |
| a sound key with no handle | 1 | **0** | **1** |
| a bus the mix table does not have | 1 | **0** | **1** |
| a cue with no sounds | 1 | **0** | **1** |
| a budget of 2, 40 plays | 40 | 0 | **0** |
| `Play::at` with no listener | 1 | 0 | **0** |
| no subscriber at all | 10 | — | nothing, no panic |

**Ten classes, ten silent consoles**: that is the defect. The crate had the sentence, wrote it into `Refused` and
`SirenLog`, and printed nothing — and the guide's *"no silent failure"* was true only of a game that had already written
the printer.

## Ranked (`plan.md` has the table)

1. **A refusal a project authored wrong is silent on the console** — chosen. The loop's own "most valuable bug" class (a
   sound that never plays and never says why), measured, and a decision the crate had already recorded with the
   measurement it asked for. It beats the runner-up (a moving owner's level) because it is wrong-by-default for every
   adopter and the runner-up is a refinement of a feature that works.
2. **The release** of the two stream-moving changes in one minor — needs the owner's explicit argument; not this
   iteration's to take.
3. **A spatial sound on a moving owner** is flat and fixed at the play (119's carry-over).
4. **The layers of a score start together** (116's design).
5. **A person listens** — and the waiting items (the log's top 5).

## After

**The matrix** (`matrix-final.log`): **9 + 247 / 9 + 256 / 118 + 565 / 20 + 262 / 129 + 593**, 7 doc, 1 readme, fmt,
clippy `-D warnings`, rustdoc `-D warnings`, `cargo build --examples` — **+4 integration tests in every build**
(`tests/console.rs`), no lib or doc test added. **Two steps did not link on the disk** — `cargo test --features semantic`
and `cargo build --examples --all-features`, at 2.3 GB free — and both were re-run alone afterwards: the first **passes
(565)**, and `cargo check --examples --all-features` compiles every example. `[[INFERENCE]]` the disk and not the code:
the same two steps failed the same way on iteration 122's tree, and the volume has 9.8 GB of dependency artifacts with
2 – 2.3 GB free. **Cue table**: 44 cases, none red. **The render set**: 21 of 21 WAVs and
every refusal text byte-identical to iteration 122's; `render.txt` identical but for the `variants` lines; `mix.wav`
**−21.5 LUFS**, LRA 0.0, **−7.5 dBFS**, its spectrogram read. **`perfcheck`**: 0.058 – 0.062 / 0.325 – 0.358 /
1.209 – 1.304 µs, inside iteration 122's band. **The public surface**: unchanged. **On the real device**: the README's
quick start with one letter wrong prints ``WARN bevy_siren: refused `footstep.grss`: no cue of that name`` once a second,
with no game code (`evidence/after/typo-on-the-real-console.txt`), and the healthy run prints no `bevy_siren` line.
