---
name: build-loop
description: One build iteration of the bevy_siren loop — pick the most valuable item, research it, build it within the crate's rules, prove it by spectrograms, EBU R128 loudness and gain plots, pass three GREAT reviews, ship and log it. Use when running a bevy_siren iteration, or when judging audio you cannot hear (it carries the four traps that have cost this project an iteration each).
---

# bevy_siren: Build the Next Most Valuable Piece

You are one iteration of a build loop on bevy_siren, a Bevy 0.20 audio crate published as 0.1.0: cues with variants,
four buses (Ambience, Dialogue, Effects, Music), `Played` / refused messages, a `problems` report, and a mix table,
wired into the Cauldron's audio panel in bevy_grimoire. Pick the single most valuable thing to build, ground it in the
literature, prove it by measurement and by looking, ship it, log it. Then stop. $ARGUMENTS

"Valuable" means: a game developer gets audio that sounds right with little authoring, fails loudly when authored
wrong, costs little to run, and can be heard and judged in the Cauldron without leaving the editor.

## Directory (every agent uses exactly this)

```
docs/loop/
  log.md            cues.yaml       needs-decision/
  runs/YYYY-MM-DD-NN/
    research.md  plan.md  eval.md  perf.md  review.md
    evidence/
      before/  after/  spectrograms/  panel/  plots/
```

`docs/loop/log.md` and `docs/loop/cues.yaml` already exist; an iteration appends to the first and does not rewrite the
second except to add a case a new behaviour needs. A run directory is created **before** any code, named by date and
the iteration's number in `log.md` (so the newest is `-03` after `-02`, never a reused name).

## 1. Orient

Read `docs/loop/log.md`, `docs/loop/cues.yaml`, `CHANGELOG.md`, `git log --oneline -12`, open issues
(`gh issue list -R bevy-coven/bevy_siren --state open`), and the README. Then run the baseline **and record exactly
what is already red**, in `runs/<dir>/eval.md` before anything else:

```sh
cargo build
cargo test
cargo test --features serde
cargo test --features mixer
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo build --examples
```

If an earlier iteration left work half done — an uncommitted change, a branch, a `runs/` directory with no `review.md`
verdict — finish or revert it first; a new item on top of a broken one measures nothing.

**Machine facts this command relies on, measured 2026-11-02 on this Mac:**

- The default `stable` toolchain is **1.94**, older than Bevy 0.20's floor of 1.97.1, so `cargo` refuses the crate
  itself. This repository pins `rust-toolchain.toml` to 1.99.0; every `cargo` call here uses it. A **stock** toolchain
  at Bevy's exact floor is `1.98.1` — `rustup run 1.98.1 cargo …` — and that is what step 7's consumer proof uses,
  because 1.98.1 *is* the floor rather than one above it. If Bevy's floor moves, update `rust-version`, the README's
  status line, and this paragraph together.
- `ffmpeg` 9.0.2 has `ebur128`, `loudnorm`, `showspectrumpic`, `showwavespic` and `astats`, so the judging in step 6
  needs nothing installed.
- Disk has run to 100 % in this project twice. Before a release build, check `df -h ~`: a Bevy release build wants
  ~2 GB and a full disk fails as `ld: write() failed, errno=28`, which looks like a code error and is not one.
- **When the disk is full, clean THIS CRATE, not its dependencies.** `cargo clean -p bevy_siren --target-dir target`
  is the fix and it is one command: measured 2026-11-03, it removed **645 files / 6.3 GB** and took the volume from
  **122 MB to 6.2 GB free**, because `target/debug/examples` alone held **4.2 GB** of this crate's own stale binaries
  and `deps` another 2 GB of its artifacts. `cargo clean -p <dependency>` is the **wrong lever** — it is what made each
  later build need a fresh copy and dug the hole deeper across two iterations. Six iterations lost steps to `errno=28`
  before this was found, and one of them shipped a red test because the suite could not link: **a baseline you cannot
  run is a blocker of the first order, ranking above every feature.** If the volume is still short after cleaning this
  crate, `~/.cache/huggingface` and other apps' caches are **the owner's to delete, not yours** — say what holds the
  space, in GB, and ask.
- **`df -g` does not exist on this macOS** (`df: illegal option`), so a guard written with it fails *silently* and the baseline runs on a full disk.
  Use `df -k ~ | awk 'NR==2{print int($4/1048576)}'` (GB). The five-feature-set test matrix leaves **~11.5 GB** of this crate's debug binaries; clean
  this crate between sets when under ~5 GB free (iteration 112's matrix script does that with a `guard` after every command).

## 2. Choose the single most valuable item

Run the fixed cases in `docs/loop/cues.yaml` against the crate as it stands and put the result in
`runs/<dir>/eval.md` (a table: case, green/red, the observation, the reason). That table — not an impression — is the
input to the ranking. A cue that is red is usually the item.

Rank by developer impact:

1. **Bugs and wrong output:** a cue that plays wrong, a refusal that should not happen, a missing refusal, a message
   dropped or doubled, a gain or pitch outside what was authored, a duck that does not recover.
2. **Missing core behaviour a game needs:** ducking and sidechain between buses, spatial attenuation, music transitions
   and layering, voice limiting and priority, no-immediate-repeat selection, pitch and gain randomisation, fades.
3. **Authoring ergonomics:** the definitions file, error messages, the `problems` report, what a host must know.
4. **Cauldron integration:** what the audio panel shows and lets you do.
5. **Performance, docs, polish.**

**A refusal is the most valuable bug of all**, because it is silent: a cue that never plays and never says why is the
hardest thing in a game to find. The `Refusal` variants are `UnknownCue`, `NoSounds`, `MissingSound { sound }`,
`UnknownBus { bus }`, `VoiceLimit { bus }`, `SpatiallyMute(position)`; a new one needs step 4's decision file.

State in one sentence why the pick beats the runner-up, and record the ranked list in `runs/<dir>/plan.md`. Carve an
oversized item to the most valuable slice and say what was carved off.

**A claim a test never exercises is a defect candidate, and the cheapest way to find the next one is to grep for it.** Search `src/`, the guides and
the README for *"can decode"*, *"is indexed"*, *"supports"*, *"costs no"*, *"the same"*, and ask of each: which test opens a real file of that kind?
Iteration 112 found that two of the five formats the scan calls audio could not be decoded, that the header parse for two was wrong, that a doc's *"costs
no new crate"* was false by 16 crates, and that a recommendation (`0.35`) was contradicted by this repository's own earlier run — each was a sentence
nobody had held against a file or a number.

## 3. Research first

Use home-still before any code (`distill_search`; if the bridge answers `401 Refresh token expired`, use
`hs distill search "<query>" --output json`, which refreshes its own token). Ask about the item's subject: game audio
mixing and ducking, loudness standards (ITU-R BS.1770 / EBU R128), voice management and virtual voices, procedural
audio variation, adaptive music, spatial attenuation models, perceptual loudness.

Fill a gap with `paper_search` then `paper_download` for the DOI. Write `runs/<dir>/research.md` with:

- **Sources**, each with its DOI, ISBN or vendored source path, and the number it reports (an attack time, a duck
  depth, a LUFS target).
- **The technique**, in the source's terms.
- **Parameters the sources report**, as a table.
- **What good sounds like**, described so a plot can be judged against it.

Then `runs/<dir>/plan.md`: the design, the modules touched, every new field on `CueTable` / `SirenSettings` /
`MusicScore` with its default and its citation (or the words **design default**), and the tests that will prove it.
**No research, no build.**

## 4. Build within the crate's rules

- **Every rate, curve and threshold is a named field on a serde type.** Every default cites its source or says
  **design default** outright. No magic constant in a system.
- **Fail loud, one path per feature.** No fallbacks, stubs or silent degradation. A bad cue is refused with a reason,
  never played wrong. A new refusal is a `Refusal` variant carrying what is missing.
- **One writer per voice gain** — `update_mix` — and the music moves targets, never sinks. A second spawn path is a
  second set of rules.
- **No `unwrap`/`expect`/`panic!` outside tests.** Audio-thread code (`src/mixer/`) does not allocate or block in
  `next()`.
- **Randomness is seeded and reproducible**: `SirenSettings::seed`, `model::Rng`, and `tests/` prove a stream repeats.
- **Bevy API questions are answered from the local source** at
  `~/.cargo/registry/src/index.crates.io-*/bevy-0.20.1/` (and `rodio-0.22.2/` for the mixer), never the web.
- **The crate knows no game.** `tests/leaf.rs` enforces the closed dependency list and forbids a game's or the
  editor's names; keep it passing and add a row only with the sentence that argues for it.
- **Changes to the public API, the message types, or the definitions-file schema go to `docs/loop/needs-decision/`**
  as one markdown file: options, tradeoffs, a recommendation, and what each option costs a consumer. Do not decide
  them. The run continues with the rest of the item, or with the next item if the decision is the whole item.
- **Consumer breakage is fixed in `bevy_grimoire`, as its own commit in that repo.** Do not touch its four known red
  palette tests (`a_person_finds_and_runs_a_command_in_the_palette`), which fail identically with the feature off.

## 5. Test

- A unit test per behaviour touched, against **the number the source reports** — a ducking curve measured at its
  attack, hold and release; a distance law at two distances; a boundary at a tempo.
- **A determinism test:** the same `seed` and the same spawn order produce an identical `Played` stream, field for
  field.
- **A test per refusal touched**, including the one that must *not* happen (a cue that should play, plays).
- Re-run the whole baseline from step 1 and the `cues.yaml` table; a case that went red is this iteration's failure.
- Measure CPU per frame at **8, 64 and 256** simultaneous voices into `runs/<dir>/perf.md`, before and after:
  `cargo run --release --example perfcheck` prints µs per frame and µs per voice for the three counts. Note the
  machine's state (a shared machine's load belongs in the numbers or they mean nothing), and say whether the figure
  is the crate's per-frame work or the audio thread's — the example measures the first, as an upper bound, because
  `update_mix` skips a voice whose gain did not move. A change must not slow a case it did not target by more than
  10 %.
- The Cauldron's panel tests: `cargo test -p bevy_cauldron --features bevy_siren --test editor
  audio_panel_reads_the_crates_file` in `~/bevy_grimoire`. Use `CARGO_TARGET_DIR=/tmp/grimoire-target` there and
  **remove it afterwards** — it is 14 GB.

## 6. Inspect it (a mandatory gate)

You cannot hear the result, so judge what can be measured and seen. `examples/render` is the harness:

```sh
cargo run --release --example render -- docs/loop/runs/<dir>/evidence/after
cargo run --release --example render -- docs/loop/runs/<dir>/evidence/before   # from a clean checkout of the base
```

It writes, at 22.05 kHz mono 16-bit so `ffmpeg` reads them without conversion:

- `cue-<name>-<variant>.wav` — the authored sample at the gain and speed a play gives it.
- `refusal-<case>.txt` — a refusal is a decision, written down rather than sounded.
- `bus-<name>.wav` — a bus's gain over time for a scripted duck: **one frame's gain repeated for that frame's
  samples**, because a gain is a step function between frames.
- `mix.wav`, `voices-<n>.wav` — the sum, and many voices at once.
- `render.txt` — peak, RMS, the gain at each scripted moment, and the bar a tempo gives.

Then produce and **look at**:

```sh
# Per cue: spectrogram and waveform, zoomed on the attack and the tail.
ffmpeg -y -i cue-<name>-0.wav -lavfi "showspectrumpic=s=900x300:legend=1:scale=log:color=intensity" spectrograms/<name>-spectrum.png
ffmpeg -y -i cue-<name>-0.wav -lavfi "showwavespic=s=900x200:colors=white" spectrograms/<name>-wave.png
ffmpeg -y -i cue-<name>-0.wav -lavfi "atrim=0:0.05,showspectrumpic=s=900x300" spectrograms/<name>-attack.png
ffmpeg -y -sseof -0.1 -i cue-<name>-0.wav -lavfi "showspectrumpic=s=900x300" spectrograms/<name>-tail.png

# Loudness: integrated, short-term and true peak per bus and for the master.
ffmpeg -nostats -i mix.wav -af ebur128=peak=true -f null - 2>&1 | tail -20
```

- **Read every image with `read`.** A spectrogram is where a click, a clipping edge, a gap, a denormal tail or an abrupt
  gain step shows as a vertical line, a flat shelf or a hole. Compare the attack and tail against the source's
  described envelope.
- **Loudness:** integrated LUFS and true peak per bus and for the master, against the target in `research.md` (a game
  mix is usually mastered lower than −14 LUFS; a single cue is not a master).
  **`ebur128` reports −70 LUFS for any clip shorter than one gating block (0.4 s)**, however loud it is, because its
  gate has nothing to integrate. So measure a short cue **looped** — `-af "aloop=loop=3:size=<samples>,ebur128"` — and
  measure its amplitude with `astats` (`Peak level dB`, `RMS level dB`). A −70 in the table means the clip was short,
  not silent; that distinction is worth an iteration, so check it before reporting a cue as quiet.
- **Bus plots:** plot a bus's gain over time as a **line** — an SVG written by a small script, not `showwavespic`,
  which draws audio and turns a gain curve into an unreadable blob. Mark the frame the duck fired.
- **The panel:** `SIREN_SHOT` and `SIREN_SIZE` capture the Cauldron's audio panel — or `examples/mixer` when the change
  is in the crate — at 1280×720 and 1920×1080, with a zoomed crop (`ffmpeg -vf crop=…,scale=…`) of the new control or
  label. A panel that needs the code explained has failed pass 3.

Write the verdicts in `runs/<dir>/review.md`, **three passes, each with a written critique before its verdict**:

1. **Function:** does it do what the source and the cue case say? Cite the case number and the source's number.
2. **Zoom:** read the plots and crops closely for clicks, clipping, gaps, tails and abrupt steps. Name the file and the
   region of each defect.
3. **Fresh eyes:** regenerate with a **different seed and a different cue set** (`--voices`, a changed table). Does it
   still hold, and can a developer understand the panel without the code?

Verdicts are **GREAT, NOT YET, FAIL**; only GREAT passes. Any fix returns you to the earliest pass it could affect. After
5 failed rounds, log the blocker in `log.md` and revert to the last GREAT state.

**A measurement changes every sentence that quoted the old answer, and every count in prose is a claim** (iteration 113 paid for all four below in one review):

- **Grep for the old answer before committing** — its number and its name — across `README.md`, `docs/guide/`, `docs/adr/`, the `src/` and `examples/` doc comments. A second golden library overturned
  `0.35` and six places still offered it.
- **Recompute every count you write by script from the evidence, never from memory of an earlier analysis.** Four counts typed into a guide were wrong (21 of 29 for 13; five classes for seven).
- **A headline statistic over a few dozen questions gets an interval before it goes in a table**: resample the *questions* (a bootstrap), and a proportion gets a Wilson interval. 29 × 18 pairs of descriptions
  are not 522 measurements; here the AUC did not separate the two libraries in one wording and a count did.
- **Write the predictions down before the run and score them, wrong ones first.** Three of eight were wrong, all in the flattering direction.

## 7. Ship

In this order: update the docs (`docs/guide/`, rustdoc, the README line, a `CHANGELOG.md` line with a `--sha`), commit,
push, open a PR (`gh pr create -R bevy-coven/bevy_siren --fill`), merge it, `git pull --ff-only`.

A visible change also ships its picture: a still or GIF from `examples/mixer`, listed in `docs/media/README.md` with
the command that regenerates it. A change nobody can see says `No clip:` and why, and names its test.

**Releasing is separate and needs an explicit `release` argument or an instruction from me.** When asked:

1. Bump the version in `Cargo.toml`, update `CHANGELOG.md`'s link block, commit, `cargo publish --dry-run`, and **read
   the tarball's file list** (`tar tzf target/package/bevy_siren-<v>.crate`).
2. **Publish from a host that has `cargo login`.** This Mac does not; `big` does
   (`~/.cargo/credentials.toml`, a `[registry]` token, which is crates.io's own section). Clone the exact commit into
   `~/publish/bevy_siren` there, publish, and check the registry answers
   `https://crates.io/api/v1/crates/bevy_siren` with the new `max_version`.
3. **Tag only the exact commit that was published, and only after the publish succeeds.** A doc fix after it is a new
   commit; the tag does not move.
4. **Prove it from outside:** a fresh crate in `/tmp` with nothing but `bevy = "0.20"` and `bevy_siren = "<version>"`,
   compiled and run on `rustup run 1.98.1` (Bevy's floor) with no path or git dependency. Paste its output.
5. `docs.rs/bevy_siren/<version>/bevy_siren/` answers 200 within a few minutes; the bare crate URL lags one sync.
6. **Never force-push, delete, or re-point a tag or branch without asking**, including to undo your own mistake.

## 8. Log

Append to `docs/loop/log.md`, newest first, under `## Iteration N — <date> — <what it is>`: the change, the rationale,
links to `runs/<dir>/` evidence, before/after numbers, the three verdicts, the commit sha, and the **top 5 remaining
items** (which is where the next iteration starts). One paragraph saying what the baseline was red on, if anything.

## Rules

- **One build per iteration:** researched, tested, measured, three GREAT reviews, shipped, logged. Then stop.
- **Nothing ships without spectrograms, loudness numbers and panel screenshots you looked at**, before and after.
- **A number without its source or the words `design default` is a magic constant**, even in a comment.
- **If blocked**, write the blocker in `log.md` with what was tried, and pick the next item; never idle a pass.
  A pass that ships nothing is a failed pass.
- **Never delete a `target/` you did not create**, and check `df -h ~` before believing a linker error is a code error.
