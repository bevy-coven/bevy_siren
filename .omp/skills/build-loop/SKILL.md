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

**A sentence held only by a fake is held by nothing.** Iteration 114 read the one sentence repeated in four files — *"both backends answer identically; only the sample path differs"* — against the *sample path*, which no test
had ever read (the mixer's tests recorded the calls a fake `AudioOutput` received). Reading `MixerBackend::play` beside `rodio`'s `repeat_infinite` showed a looping voice ignored every gain change after its first pass:
layered music, a fade and a stolen loop's fade were silent failures on `--features mixer`, and all 33 cue cases were green. When a claim is about **sound**, find the test that reads a sample; if there is none, that is the item.

**A test of a thing that loops needs a loop of every format the crate says it plays, and must compare a length.** Iteration 115 set out to make an Ogg loop sample-exact and found the first measurement did not
match the plan: `decoder.repeat_infinite()` of a real Ogg produced **0 of 147 456 samples**. `Cargo.toml` selected `rodio`'s `symphonia` Vorbis decoder, which reports a span of length zero until its first sample, and
`repeat_infinite` — Bevy's own loop and the mixer's — reads that as an empty source: `Played` was written, the voice ended at once, nothing was heard, on both paths whenever `mixer` or `semantic` was on, from the first
commit. Every loop test used a WAV. The claims audit had asked *"does any test read a loop's length?"*; the question that finds this is **"does any test loop a file of each format at all?"** (`grep` the tests for `Mode::Loop`
and read what the sound is). And an assertion of `first == second` over a loop that is empty holds vacuously: **assert `first.len() == expected` first.** Cheap check for any future decoder or `rodio` change:
`rodio::Decoder::builder().with_byte_len(n).with_data(Cursor::new(bytes)).build()?.repeat_infinite().take(3 * pass).count()` against `3 * pass`, on one real file of each format (`tests/ogg_loops.rs` does it for Ogg).

**A number measured on one decoder is not a number about another.** Iteration 112's 59-of-209 Oggs decoded long was `symphonia`'s, built from a `File`; this iteration's 253-of-254 was `symphonia` built with `Decoder::new(Cursor)` and 51-of-254
was `lewton`'s. A prediction that carried the first into a claim about the second was wrong (`runs/2026-11-03-115/research.md`, P5). Say which decoder, built how, on which set, every time a count goes into a doc.

**A test fixture that is always in the happy state cannot see the other states.** Every headless test's handle is `Handle::default()` — a Uuid handle the asset server answers `None` for — so a file that **failed to load** was a state no
test could build, and for the whole life of the crate `Played` was written for a sound that could not sound (iteration 116: six plays of a misspelt file, six `Played`, a log of *played* and *stolen by `x` (cue)*). When a claim is
about a thing with **states** (an asset: loading, loaded, failed; a voice: waiting, playing, ended; a device: present, absent), `grep` the tests for how each state is *built*, and where a state has no builder, that is the item.
Make the state **deterministic** — a loader that **waits at a gate your test opens**, then succeeds or fails as told; not a file that is missing (a race with the disk), and not a sleep (a race with the next frame) — and measure the discovery on the real thing (the real `AssetServer`, the default cue)
before writing a prediction.

**A crash a headless app cannot show is still a crash, and the way to find it is to read the `unwrap`s of the code your entity is handed to.** Iteration 117: a voice is an `AudioPlayer`, and Bevy's `AudioSource::decoder` is
`….build().unwrap()` (`bevy_audio-0.20.1/src/audio_source.rs:101`); a file that loads and is not audio the build decodes (a Git LFS pointer, an Ogg *Opus* file) was `Played` and then **panicked the game in the same frame** — on the default
build, for the crate's whole life — while four sentences said it was refused, and no test could see it because a headless app has no device and so never runs Bevy's decoder. `grep -n "unwrap()\|expect(" <dependency>/src` over the path a
spawned entity takes finds these in minutes. **To see one, run the real plugin on this Mac's device in a scratch program — one process per case, `catch_unwind` around `app.update()`, the panic hook printing the location.**
**A verdict that is a property of the build is tested as a property, against an oracle, not as a table**: `tests/undecodable.rs` asserts the crate refuses a play *exactly when `AudioSource::decoder()` panics* (`catch_unwind`), for every file, so
it is true with and without `mixer` (an MP3 is refused where `rodio` has no decoder for it and played where it has). And when a check must be *the same call* as the code it protects, **reach it through the trait's associated type** (`<AudioSource
as Decodable>::Decoder::builder()`) before adding a dependency: it was the whole difference between a new dependency in every consumer's default build and none.

**Run the documented use of a handle the crate hands out, not only its refusals.** Iteration 118: `Played::entity` is documented as *"for a game that hangs its own data on it"*; a probe that did exactly that — a system after `SirenSystems::Play`
doing `commands.entity(played.entity).insert(..)` — found that on the crate's own mixer bytes `rodio` refused were `Played` **and** `Refused` in one frame, the entity despawned in it, and the game **panicked in `bevy_ecs`'s command handler** (Bevy 0.20's `insert`
on a despawned entity is an error whose default handler panics; only `try_insert` is silent). A refusal test that counts messages and voices never touches the handle. For every identifier a message hands out (an `Entity`, an index, a key), write the
one line a game would write and run it through the scenarios that **end** the thing it names: refused, stolen, stopped in the frame it started, owner gone, load failed. Then state the **invariant you can honestly promise** and test it across them
(`tests/one_answer.rs`: *each play is answered once, and a `Played` entity that is gone is explained by a `stopped` line*) — the first form, *"a `Played` entity exists when the frame ends"*, was red for an **intended** behaviour (a voice stolen in the frame it
was announced never sounded and is despawned in it), so the promise was narrowed and the exception written where a game reads it (`Played::entity`'s doc). **Mutate the guard to see which test notices** (hook ignored, answer ignored, one memo for two askers, no
memo, the real backend's check always `None`): five switches, each reddened the tests that guard it and no others needed.

**A relationship is three things at once — a lifetime, a transform and a hook — and a crate that wants one of them gets the other two.** Iteration 119: `Play::on` made the voice a `ChildOf` of its owner to get *"ends with the owner"*, and running the documented use across
the **states the owner can be in** (spawned; allocated and not yet spawned; gone; no `Transform`; at the place asked for) found four defects from that one line: a **panic** (the hook queues a command on an entity that does not exist yet — every `Commands::spawn` returns one),
a sound at **owner ∘ `at`** (12 dB under the law, or **+52 dB over the sample** for an owner with no `Transform`: Bevy never propagates a child of a parent that has none, so the crate's +80 dB panner compensation stood where the panner did nothing), a **loop that lived for ever**, and a
**false log line**. `grep -n "ChildOf\|add_child\|with_child\|set_parent" src` and ask of each use which of the three it wants. `Entities::get_spawned` tells **alive, pending and gone** apart in a plain system (`&Entities` is a `SystemParam`), and `world.entity_allocator().alloc()`
then `world.spawn_at(id, ..)` builds the pending state deterministically for a test — an unordered system in a probe reproduces it too, but only by the luck of a topological order. **Ousterhout's *define errors out of existence* (§10.3) and Hunt and Thomas's *crash early* (tip 38)
are the pair**: crash for what was supposed to be impossible, define away what is merely unusual; a pending owner is the ordinary state of every entity in the frame it is created. And **a sound that will never be heard must take nothing from one that is** — the first cut let a play on a
gone owner steal a playing voice before it ended (`doomed` in `play_requests` is the fix; the test for it is red when the exemption is switched off and no other is).
**A README's code is a program: paste it, run it on the real thing, and read the console the reader reads.** Iteration 120: the Quick start — the first code every adopter runs — **never played a sound, not once since the first commit**. Pasted verbatim as an example, the reader's own
`cargo build` said `function walk is never used` and `function hear is never used` (`main` added neither); run for seven seconds on a real window and device it played nothing and printed one line, Bevy's `Path not found: …/audio/step_01` (the keys had no extension and `load_table`
asks for `<dir>/<key>`); and its `hear` wrote a second log entry stamped `0.00s`. `tests/readme.rs` had *compared the README with a copy of itself*, behind a feature nobody enables — **a text comparison is not a test of a program.** The claims audit for a front door is: **paste each
code block verbatim into a scratch example, build it (read the warnings), run it with the real plugins, and read everything the console says**; then run every command the README tells a reader to type (`music`, `mixer` with `SIREN_FRAMES`, `budget` all held). The fix that holds it: **one
file is the block** (`examples/quick_start.rs`), `include!`d by a test that runs its own systems on the real asset server over `assets/` and Bevy's real `AudioLoader`, with **the console captured by a thread-local `tracing` layer** (`bevy::log::tracing_subscriber`, no new
dependency; `with_default` sees only its own thread, so force `Update` single-threaded with `SingleThreadedExecutor`), and the README and the crate-root doc **compared to that file** — plus a static check that **`main` names every function the block defines**, because the behavioural tests wire the
systems themselves and cannot see a block that forgets one (mutation M1). A **negative control** through the same code (the old keys) proves the failure is *loud*. And **a log is read with a mark, not a position**: `seen = log.len()` printed 256 of 600 entries and went quiet for good, so
`SirenLog` carries each entry's push number (`pushed()`, `since(mark)`). A freshly linked 520 MB binary may print nothing for ten seconds on its first launch: run it again before believing it is silent.
**A seed is a name, not a state: test a generator as a function of its seed, across consecutive seeds.** Iteration 121: `Rng::new(seed)` was xorshift64 whose state *was* the seed, and **no test, render or probe had looked at what a small seed opens**, because every one used the default seed (bits spread) or asserted a property of a
*whole* stream with the seeds 3, 5, 7, 11 and 99. Asked, with a ten-line probe over seeds 1 … 3000: **the first pick of a cue was its first variant 3000 times in 3000** (`Rng::new(1).next_u32() == 0`), neighbouring seeds' first draws correlated at **0.9995** and their draw *n* to n = 64, one flipped seed bit changed 3.8 of 32 bits of the first
draw, and the streams of the seeds *a*, *b* and *a* ⊕ *b* were xor-related for all 1000 draws, in 1953 of 1953 triples — a **linear** generator seeded linearly (Brent 2007 §4: *"clearly undesirable"*; Blackman and Vigna 2021: the cure is a nonlinear step). The claims audit for any seeded behaviour: **run it for seeds 1, 2, 3 … and ask four things of the
opening — is it uniform (χ² against its 0.1 % point), is it uncorrelated across neighbouring seeds (Pearson r, within 4/√N), does every seed bit reach it (the mean avalanche is 16 of 32 *and each position is inside [15, 17]*: a bare multiplication passes the mean and fails bit 39), and are related seeds unrelated streams (xor)** — and hold the mixing function itself to **known answers from an independent implementation**
(a one-bit constant typo and a one-round mixer pass every statistic: mutations M4 and M5). **A change to a seeded stream is a release note, not a patch** (`needs-decision/exact-pitch-draw.md`, `seed-scramble.md`): ship it with the other one that moves the stream, pin the first draws of several seeds, and **never let the reported seed differ from the written one** —
`seed_choices` compares them every frame, and a reported seed that moves restarts the stream each frame (M2: nine play tests red). **Check the disk before a matrix: building every example with every feature takes ~3 GB of debug binaries**, and `cargo clean -p` cannot see the binaries of a scratch example whose source is gone — remove those by hand.
**Run `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings` after every edit, before any test and before the matrix** — it was the first red of the matrix in iterations 115 and 116, and the cargo output a tool summarises shows `Finished`
and hides the warning that becomes the error.

**Tool discipline is part of the loop, and the user has said so twice.** Read with `read`, `grep` and `glob`; bash runs **programs** (`cargo`, `ffmpeg`, `git`, `python3` as an analysis, `cp`/`rm` of your own scratch). Not `cat`,
`head`, `tail`, `ls | awk`, `sed`/`perl`/`python` to edit (use `edit`), `tee` or `>` to keep output: a bare command's full output is `artifact://<id>` — `read` or `grep` it, and `cp artifact://<id> <evidence file>` keeps it. After
`cargo fmt` the line numbers move: **re-read before the next `edit`**. A file read in the same turn as the bash that creates it can race it: read it again.

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
ffmpeg -y -i cue-<name>-0.wav -lavfi "showwavespic=s=900x200:colors=blue" spectrograms/<name>-wave.png   # NOT colors=white: white on a transparent PNG is a blank image to `read`
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
