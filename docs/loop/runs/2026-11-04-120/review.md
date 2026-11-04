# Iteration 120 — review

Three passes, a critique written before each verdict. Verdicts are GREAT, NOT YET, FAIL; only GREAT passes.

## Pass 1 — Function (cue case 41; Uddin and Robillard 2015; Hunt and Thomas, Tip 13 and the Starter Kit)

**What passed.** The block a reader copies, pasted and run on a real window and this Mac's output device, **plays**: ten steps alternating `step_02.ogg` / `step_01.ogg`, each `played` and then `stopped: ended` a clip's length later (0.24 s for the 0.221 s file, 0.26 s for the 0.261 s one), every line the crate's own
sentence, once (`evidence/after/quick-start-real-run.txt`). On the unchanged tree the same paste printed two compiler warnings (`function walk is never used`, `function hear is never used`), played nothing, and put one line on the console — Bevy's `Path not found` — that names neither this crate, a cue nor a key
(`evidence/before/readme-as-written.txt`). The tests run the block's own `setup`, `walk` and `report` on **the real asset server over `assets/`, Bevy's real `AudioLoader` and the real files**, with **the console captured** (`evidence/after/quick-start-console-in-the-tests.txt`): eight steps in four seconds, the two recordings alternating,
none refused, the console equal to the crate's log in order and each line once; the old keys through the same `report` print ``refused `footstep.grass`: the file for sound `step_02` failed to load: Path not found: …/assets/audio/step_02``. **Final matrix** (`matrix-final.log`): 9 lib + **233** (was 226) + 7 doc, serde 242, semantic 551 + 118, mixer 248 + 20,
all-features 579 + 129, doc 7, readme 1, clippy `-D warnings`, fmt, rustdoc `-D warnings`, both `cargo build --examples`, all green; the only warnings are the macOS linker message the `mixer` example always had, now also on `quick_start`.

**Mutations** (each switch applied to the final tree, the named tests run, then put back; `evidence/before/mutations.txt`):

| switch | red | stays green, and why |
|---|---|---|
| M1 `main` stops adding `walk` | `the_quick_start_adds_every_system_it_defines`, `the_crate_root_doc_is_the_example` | **the two console tests** — they wire the systems themselves; this is the reason the static check exists |
| M2 a key loses its extension in the example | `the_quick_start_plays_both_recordings…` (4 plays, all `step_02.ogg`; the console names `step_01` and its path), the doc comparison | `a_key_without_its_extension…` (it brings its own old keys) |
| M3 README block edited | `readme::the_readme_quick_start_is_the_example` only | everything else |
| M4 crate-root doc block edited | `the_crate_root_doc_is_the_example` only | everything else |
| M5 `since` skips the entry numbered `mark` | both cursor tests that read across a mark, and both console tests | the eviction test (its mark is an evicted entry's number) |
| M6 `since` goes by position, not by number | `since_is_exact_when_the_ring_evicts_an_entry_after_the_mark` **only**: `["r254", "c2"]` for `["c2"]` | all the others, including the 1 000-entry wrap — every entry there is routine, so position and number agree |
| M7 `report` never moves its mark | the plays/console test (printed twice), the doc comparison | |
| M8 `clear` rewinds the count | `the_count_survives_a_clear…` only: `(0, 0)` for `(0, 5)` | everything else |

**Critique, first reading** (each is a thing that could be wrong, and what was done about it):

1. **The console tests are headless, so they print `stopped … stolen by footstep.grass (cue)`, not `ended`.** With no device a voice never gets a sink and `retire_voices` does not treat *no sink* as *ended* (AGENTS.md), so the cue's default `max_instances` of one takes the previous step back. The device path — `ended` — is shown by the real run, which is one run by hand,
   not a test: the repository's device tests return early without a device. Said in the evidence file and here; the tests do not assert on `stopped` either way.
2. **The first step of the real run is at 2.06 s, not 0.5 s.** The first frames of a window are slow and `Time<Virtual>` clamps a frame to 250 ms, so `walk`'s half second of virtual time took two real seconds. A reader sees the window, waits two seconds and then hears steps. Using `Time<Real>` would give 0.5 s but would
   make the block's footsteps keep walking through a pause, which is the wrong thing to teach (the crate's own mix reads real time on purpose; a game's gameplay does not). Left, and written down.
3. **`report` prints at `info!`.** A game that filters `info` away sees nothing, and a refusal is the one line that should not be missed. A `warn!` for refusals in the block would be louder; but the *crate* printing authoring refusals is a design question (`needs-decision/log-cursor.md`, option 4), and the block is the game's own code. Not decided here.
4. **`unscheduled` is a textual check** — it assumes `fn main() {` and a closing `}` at column 0 and matches names as words. It would be wrong for a block whose `main` wires systems through a helper. For a 37-line file it is enough, and it is held by its own negative fixture, which is the old block's shape.
5. **`include!` puts the example's `main` and `use` lines into the test module.** An inner attribute or a `//!` in the example would stop it compiling — loudly, and the test's header says why. `#![allow(dead_code)]` covers the unused `main`.
6. **The new public surface is two methods** (`SirenLog::pushed`, `since`), additive, recorded in `needs-decision/log-cursor.md` with four options; the representation grew by 8 bytes an entry.
7. **Not proved from a fresh crate outside the repository.** A second Bevy build would not fit this disk (3 GB free); the example is the README's text and compiles against the path crate. What a reader's `Cargo.toml` needs is Bevy's default `vorbis` feature for `.ogg`; a Bevy with `default-features = false` would fail the load with *no loader*, which the log
   would say, and **no test builds that**.
8. **The before cost of `push` was not measured** (a second worktree needs a second multi-gigabyte target directory). The after is 3 – 5 ns on top of ~30 ns to build the entry (`perf.md`).
9. **A false sentence of mine, caught by reading it again:** `cues.md` said the old mistake was *"the one mistake a first project makes"* — unmeasured; now *"an easy mistake"*. And `AGENTS.md` said the Quick start never played *"for the crate's first 119 iterations"*; `git log -S` says it is in the first commit (`22c39a4`, 2026-11-02), so it says **since the first commit**. And `cues.yaml` case 41 said eight steps where the test asserts six (eight happen).
10. **The Cargo.toml comment on `readme-example` still said the feature compares the README with the crate-root doctest** — found by grepping for the old answer; corrected.
11. **A readability catch of the first matrix:** the block's `walk` kept a local named `since` two lines above `log.since(*seen)`; renamed `elapsed` in all three copies and the matrix run again.

**Predictions** (`research.md`; wrong ones first):

| | prediction | result |
|---|---|---|
| P9 | the footsteps are broadband under 2 kHz and **nothing above 4 kHz** | **right in level, wrong in letter**: the energy (−10 … −50 dBFS) is under 2 kHz, but the spectrograms show faint components up to ~9.5 kHz at −75 … −100 dBFS and a −100 dBFS broadband band in the first 15 ms of `step_01` (the 4 ms fade-in's leakage): 94 dB under the peak, inaudible, and not "nothing" |
| P7 | `perfcheck` inside its 10 % band | held, **and it does not test the change**: `perfcheck` never pushes an entry; `push` and `since` were timed directly, after only (`perf.md`) |
| P8 | each guard reddens its own test and nothing else needed | held, with a surprise in each direction: M1 reddens **no** behavioural test (by design), and M7's first form of the assertion died on a slice index instead of saying what was wrong (now a sentence) |
| P1 | ≥ 6 plays, both files, no refusal, `played` lines on the console | held (8 plays), plus the headless `stolen by` lines (critique 1) |
| P2 | the old keys print `failed to load` and `assets/audio/step_01` | held (`step_02`, `step_01` alternating, eight lines) |
| P3 | the `len()` cursor stalls at 256 | held: **256 of 600**, the last `c255` (`evidence/before/len-cursor-stalls.txt`) |
| P4 | `since` exact when an entry after the mark is evicted from the middle of the ring | held; M6 shows the one test that sees it |
| P5 | under a second, no flake in 20 runs | held: 20 of 20 at the default thread count (0.05 – 0.09 s) and 5 of 5 at one thread |
| P6 | passes under `mixer` and `semantic` | held (248 and 551 in the matrix) |

**Verdict, pass 1: GREAT.** The function is what case 41 says, run on the real thing, with the failure it replaced reproduced and the cause of the failure surviving removed (a test that runs the block).

## Pass 2 — Zoom (the plots and crops, read closely)

The only audio this change adds is the two footsteps a reader hears first, so they were read as images, at the attack and the tail, and measured (`evidence/spectrograms/`):

* `step_01-spectrum.png`, `step_02-spectrum.png` (0 – 0.23 s / 0 – 0.27 s): the energy is a warm wedge under about 2 kHz, brightest in the first 40 ms (−10 … −30 dBFS), purple (−70 … −90) by 0.1 s and black by 0.2 s; `step_02` is darker and longer, as built. **Defects named:** *(a)* `step_01`, 0 – 15 ms, all frequencies to 20 kHz: a faint blue vertical band at about −100 dBFS — the
  fade-in's broadband leakage, 94 dB under the peak, not audible; `step_02`'s is weaker (−115). *(b)* both, 0 – 0.1 s, horizontal faint lines near 8.8 and 9.5 kHz at −80 … −90 dBFS: the encoder's band-edge structure, inaudible. No click, no shelf, no hole.
* `step_01-wave.png`, `step_02-wave.png`: a sharp onset at about 12 ms, the amplitude gone by about 0.10 s of 0.22 s — the exponential fade is steep — and no flat-topped peaks. Peaks **−6.16 / −6.10 dBFS** (`astats`), crest factors 7.7 / 8.2.
* `step_01-attack.png`, `step_02-attack.png` (the first 50 ms): a smooth rise over the 4 – 5 ms fade-in at the low frequencies; no vertical line across the spectrum at the first sample except the −100 dBFS band of (a). (The time ticks read `0.00s 0.00s 0.01s…`: `showspectrumpic` rounds the labels of a 30 ms image; the data is fine.)
* `step_01-tail.png`, `step_02-tail.png` (the last 100 ms): dim low-frequency energy at −70 … −100 dBFS fading to black; no abrupt end, no tone, no hole.
* **Loudness**: eight seconds of the walk (the two alternating every 0.5 s, looped) read **−24.0 LUFS integrated, LRA 0.0** — nothing like the −14 LUFS of a master, and quiet enough not to startle a reader who runs the example with the volume up. A single step is shorter than ebur128's 0.4 s block, so it was measured **looped**, as the skill says.
* `mix.png` (the render's sum): **21 of 21 WAVs byte-identical to iteration 119's**, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak; an onset in the first 30 ms, three steady low lines, no vertical line, hole or shelf after it.
* `docs/media/quick-start.png` (2297 × 616): legible at the size the README shows it, three blocks (before, after, the old keys), nothing composed.

**Critique.** Whether the two files **sound like footsteps** is a judgement nothing here can make: they are filtered noise bursts, which is how a synthesized step is made, and the README calls them placeholders. The level, the spectrum and the ends are measured; the *character* is for a person (carried). The −100 dBFS leakage band is real and inaudible; a better fade shape would remove it, and it
was not worth a second pair of files. **Verdict, pass 2: GREAT.**

## Pass 3 — Fresh eyes (a different seed and a different cue set; can a developer understand it without the code?)

* **A different seed, cue set and asset directory** (`evidence/after/fresh-eyes-seeds-and-cue-set.txt`): the block's own `walk` and `report`, an asset directory in `/tmp`, **three** variants with `gain_db −4` and a ±1 semitone pitch range, and **five seeds** (1, 7, 42, 99, 123 456): 16 steps in eight seconds under every seed, 0 refused, never a recording twice running, all three
  heard, the sequences different from seed to seed. It holds.
* **What it surfaced, and is not this item's** — all five seeds began with `step_01.ogg` (odds 1.2 % by chance). Asked of the real play path for **sixty seeds, one play each**: **60 of 60 first picks are the first variant**, and the first detune is **`−1 + 0.125 × seed`** — the first draw is a linear function of the seed. `Rng::new` uses the seed as the xorshift64 state with no scrambling and `next_u32` returns
  the top 32 bits, so for a small seed — the natural `seed: 42` — the first play is the first variant at a pitch the seed sets. The README's *"two seeds are two streams from the first play"* holds for the stream and not for its beginning. The default seed is a large constant and unaffected. **Not fixed here**: it changes every seeded sound the crate makes (the 21 WAVs, a saved
  project's replay) and needs its own research, decision file and re-baseline. It is **the first item of the next iteration**.
* **Can a developer understand it without the code?** The block's four comments say what a cue is, what a key is (*a file's name under `assets/audio/`, extension included*), what `walk` does and what `report` is; the README sentence under it says where the files go, what the console will read (a played line, and the refusal line for a missing file, with its path) and what holds the block. A reader who mistypes now sees the cue, the key, why and the
  path; one who pastes it with no files sees `failed to load` for each. **A reader who runs it from a clone hears steps after about two seconds** (critique 2) and sees the lines in the terminal.

**Critique.** The fresh-eyes run was a scratch test, deleted; the claim it supports is kept in the evidence file, not in a permanent test (a permanent one would pin the RNG's incidental first draws — exactly what the next iteration changes). **Verdict, pass 3: GREAT.**

## What this iteration caught about itself

*The block's `since` local read like `log.since` two lines below it* (renamed, matrix re-run). *The Cargo.toml comment, `cues.yaml` case 41's threshold, `AGENTS.md`'s *"119 iterations"* and `cues.md`'s *"the one mistake"* were each stale or unmeasured* (all corrected). *M7's first assertion died on a slice index* (now a sentence). *The first launch of the freshly linked 520 MB binary printed nothing for nine seconds* — a first-execution delay of the machine, not the code — and the second launch, kept as the evidence, is complete. *The disk reached 1 GB:* the first
final matrix failed two large links at 2.7 GB free (**[INFERENCE]** the disk: they linked at 3.5 GB and, incremental off, at 3.7 GB) — handled by cleaning this crate's own release artifacts and rerunning (`matrix-first-attempt-disk.log`, `matrix-final.log`).

## Tool discipline

Files were read with `read`, `grep` and `glob` (batched in parallel, one round trip for several), edited with `edit` (several files in one call: three copies of one rename, the README, `lib.rs`, the example and `Cargo.toml` in one), and bash ran programs: `cargo`, `ffmpeg`, `git`, `python3` as the repository's transcript renderer, `cmp`, `cp`/`rm` of my own scratch and build products, and `df`, `du` and
`uptime` for machine facts. Where a read tool would have done — `ls -l` of a binary and of a directory listing, twice — I used bash; that is the whole of the deviation I can find. A bare command's full output was read from its artifact (`cp artifact://5423` keeps the red build); the matrix's own output, which arrives only with `wait`, was written to the run directory with `write`.
