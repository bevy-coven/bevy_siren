# Iteration 116 — review

Three passes, each with its critique written **before** its verdict. Verdicts: GREAT, NOT YET, FAIL. A fix returns to the earliest pass it could affect.

## Pass 1 — Function

*Against the item's own acceptance — a play whose file failed to load is `Refused` with the cause and plays nothing; a voice whose load fails after it was played is refused in the frame the server says so and freed; a load still in flight is not refused — and against the cue table: 37 cases (36 held, one new), none red; the 21 rendered WAVs are byte-identical to iteration 115's (21 of 21).*

**Critique, first reading — seven things that were wrong in my own work:**

1. **My first cut refused a play whose file's *reload* had failed even though the previous bytes were loaded.** A failed hot reload leaves the server's state `Failed` and the old bytes in `Assets` (a failed reload removes nothing); `answer` read the state alone, so a cue the game could still sound was silenced for a file the developer was half-way through saving. I found it by asking what `Failed` with bytes *present* means, after five green tests; the sixth test went red on the first cut (`evidence/before/reload-test-red-on-the-first-cut.txt`) and `sounds::cannot_load` — *failed **and** nothing to play* — replaced the bare predicate at both call sites. My discovery probe used a file that never existed, and the prediction table had the shape of that probe (`research.md`, scorecard).
2. **The first matrix of this tree was red, on clippy alone — and I had not run clippy before it, for the second iteration running.** The iteration-115 review said clippy would be the first command after an edit; it was not. The cause was `private_interfaces`: I had made the asset server and the asset table one `#[derive(SystemParam)]` struct to keep `play_requests` within Bevy's sixteen parameters, and a crate-private type in the signature of a `pub fn` is an error under `-D warnings`. It had been a *warning on every build line* of that matrix (`1 warning: bevy_siren (lib) generated 1 warning`); my own cargo runs came back through a summary that shows `Finished` and the test lines, so I never read it. A type alias for the tuple of the two resources is one parameter and adds no type to the signature. After that fix every edit was followed by `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings` *before* a test, and the whole matrix was run again — twice, once for the alias and once for the tests below.
3. **A premise of my tests was a race, and I found it by reading the file, not by a flake.** The loader slept 100–150 ms to hold a load "in flight", so *"the load is still in flight when the play is asked"* was a race between that delay and the next frame — false on a machine starved for 150 ms — and in the control test merely vacuous (a file that had already loaded would still have been played). A gate the test opens (`Gate`, `Gated`) replaced the sleep; the control now holds the load open for ten frames and asserts nothing is refused and the voice waits; the music test asserts both layers' voices are waiting before the gate opens. 23 of 23 runs green, eight of them with every core held by a busy loop of my own (`evidence/after/gated-loader-stability.txt`). **I did not run the sleeping version under the same load, so this is a removed race, not a measured flake rate.**
4. **Mutation checks first ran on the first cut.** They were run again on the final code (play-time check off, in-flight system out of the chain, bytes-present guard off — `evidence/before/final-mutation-*.txt`) and, after the gate, again for the first two on the gated tests (`evidence/after/gated-mutation-*.txt`): each reddened exactly the tests that guard it (one of six for the play-time check, three of six for the in-flight system, one for the guard). The fourth, *a load in flight counted as failed*, was run on the first cut only (`evidence/before/mutation-loading-counted-as-failed.txt`); `load_error` has not changed since, and the control test is stronger now, but **I did not run it again**.
5. **The cost was measured on the first cut.** The scratch benchmark ran before the `Files` refactor and before `cannot_load` looked in `Assets` first; the shipped common path does less, and `perf.md` and the CHANGELOG say the numbers are an upper bound. `perfcheck` runs model arithmetic only and cannot see the new system (it says 0.059–0.064 / 0.336–0.355 / 1.189–1.245 µs a frame at 8 / 64 / 256 voices, which is a statement about the model and not about this change).
6. **A multi-file `edit` call is not atomic.** One call applied `playback.rs` and refused `sounds.rs` and `play.rs` (stale tags after `cargo fmt`), leaving the tree not compiling for a moment; I saw it in the tool's output and re-read before re-applying. Not a defect that shipped; a thing to do differently: re-read after `cargo fmt`, before the next edit, always.
7. **The discovery probe came before the predictions**, so P1 is a regression statement and not a prediction. Said in `research.md`; the iteration-115 lesson (*predictions before measuring*) was kept for the *numbers* and not for the finding.

**What passed.** Probe on the unchanged tree: six plays of a misspelt file → six `Played`, no `Refused`, a log of *played* and *stolen by ui.click (cue)*, three dead voice entities alive (`evidence/before/failed-load-probe-on-the-unchanged-tree.txt`). Same probe after: 0 `Played`, 6 `Refused`, 0 voices, 0 counted on `Effects`, and every log line reads *"the file for sound `typo_click.ogg` failed to load: Path not found: /tmp/siren-stems/typo_click.ogg"* (`evidence/after/failed-load-probe-after.txt`). On the unchanged tree **three of the four first tests are red** (four of five with `mixer`), the control passes. The final matrix (`matrix-final.log`, the third run of this tree): build, `cargo test` 208, `--features serde` 217, `--features semantic` 526 + 114, `--features mixer` 222 + 16, `--all-features` 553 + 125 (the baseline was 203 / 212 / 521 + 114 / 216 + 16 / 547 + 125: five tests more on the rows without `mixer`, six on the two that have it), doc tests, readme, both example builds, clippy, fmt and rustdoc — all exit 0. The only warning left is the `example "mixer"` one the baseline already had. Render: 21 of 21 WAVs byte-identical, `mix.wav` −21.5 LUFS, LRA 0.0, −7.5 dBFS true peak, spectrogram read (`evidence/spectrograms/mix-spectrogram.png`).

**Verdict: GREAT.** After the fixes above, the item's acceptance is met on both backends and its guards are mutation-checked.

## Pass 2 — Zoom

*The source diff, `git diff src/` and `tests/failed_load.rs`, read line by line as a reviewer who did not write it; and every sentence I wrote about it.*

**Critique, first reading — what a line-by-line read found:**

1. **`SirenSounds::failed`'s doc said a play whose sound is in that list "is refused"** — false for a key whose reload failed and whose bytes are loaded (it is listed and still plays). Now says *"and whose bytes are not loaded"* and names the exception.
2. **`cues.yaml` case 37 said a dead music stem is "refused twice (the voice, then the retry)"** — a count the test does not assert (it asserts: refused with the cause, then nothing written for 60 frames). Rewritten to what is pinned. The CHANGELOG and case 37 each also counted the red tests wrongly once (*"three red … four"* where the sixth test is red only on the first cut); corrected.
3. **`research.md` was headed "the wrong ones first" with none wrong, and closed on "six of six" for five predictions.** Rewritten: none was wrong; the finding is what the table left out.
4. **Stale words after the refactors**: `Files::cannot_load` in `perf.md`; the sleeping `Slow` loader in `AGENTS.md`, the skill, the CHANGELOG, `cues.yaml` and `docs/media/README.md`. All now say gate / `sounds::cannot_load`.
5. **`docs/guide/music.md` said nothing about a dead stem**, and `MusicState::blocked` cannot say *why* a layer waits — a panel reads *"waiting for a voice"* for a layer that waits for a file. Added a paragraph; the log line is the answer.
6. **`refuse_failed_loads` also refuses a voice that is already `Stopping`** (a play stolen while it waited writes *"stopped … stolen"* and then *"refused … failed to load"* for one voice). I left it: both lines are true, and a filter would be one more state to keep right for a double line nobody reads.
7. **`Refusal::MissingSound`'s `Display` still says "no handle or undecodable bytes"**, which a game printing `Refused.reason` reads for a typo'd file. Left on purpose — `tests/play.rs:295` pins the wording and a new variant is a public change (`needs-decision/undecodable-sound.md`, option 1, which now argues for *two* variants). The cause is in the log, which the picture shows.

Read and found as intended: `answer` checks the handle, then the file, then the bus — a failed file is refused before it can claim a budget slot, and no `Played` is written; `play_requests` has one parameter more (the tuple of the two resources), which a consumer who registers it as a system does not see; `refuse_failed_loads` runs after `start_pending` and `trim_ogg_loops` in the same chained set, and its query is `Without<MixerVoice>, Without<AudioSink>, Without<SpatialAudioSink>` — a voice a backend has started is none of its business; the `Gated` loader gives up when its test is gone (a panic) or after ten seconds, so a failing test cannot hold a pool thread from the tests after it.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes

*Not the item's own cases. What would a game that did not read this iteration's notes meet?*

**Critique, first reading:**

1. **A cue of three recordings, one misspelt.** Refused whenever that variant is picked (a third of the plays), as a key with no handle already was — skipping it would play two thirds and hide the third. By reading, not run.
2. **A looping ambience whose file failed** is refused at the play: no voice, no zombie loop. A music layer is the tested case.
3. **`Play::on(owner)`** with a failed file: refused at the play, no entity spawned; in flight the *voice* is despawned and the owner untouched. By reading.
4. **A world with no `AssetPlugin`** has no server and no table; `cannot_load` returns `None` at its `?` and nothing is refused. By reading — the headless tests all add the plugin, with default (Uuid) handles the server has no record of, which is also why none of them could build this.
5. **The semantic library's `audition`** returns a `Play` for the caller to send (the tool host answers `unavailable`, as ever), so a failed library file is refused with the cause through the same path; not exercised with a failed file (`--features semantic`: 526 + 114 green).
6. **A load that never resolves** (`Loading` for ever, a handle somebody reserved and never filled) still waits for ever: nothing says how long a slow disk may take and a timeout would be a number without a source. And **bytes that load and do not decode** still panic Bevy's own path (`AudioSource::decoder` unwraps) where the mixer refuses them; carried.
7. **The Cauldron's audio panel was not run** (the editor pins `82d227e`, 221 commits back, and a 14 GB target on a disk with 5.7 GB free); no API the panel uses changed.
8. **Nobody listened — there is nothing to hear.** A refused play makes no sound, and the render's WAVs are byte-identical.

**Verdict: GREAT.** The limits above are recorded, with their routes, in `docs/loop/log.md`.

## Prediction scorecard

In `research.md`: five predictions, five right as written (P1 was a regression statement). The finding the table did not contain — a failed reload with bytes loaded — is change 1 above.

## Tool discipline, as asked

The standing instruction is to read with `read` and `grep` and use bash for programs. Every file read and every search in this pass went through `read`, `grep` or `glob`; every edit through `edit` or `write`. Bash ran `cargo` (build, test, clippy, fmt, doc, the matrix), `ffmpeg`, `git`, `python3` for the transcript picture, `cp`/`rm` of my own scratch (`cp artifact://… evidence` keeps a run), and `sysctl`/`yes`/`pgrep` for the stress run — `yes` processes of my own, killed by pid and checked gone. **The avoidable ones:** an `ls -l | awk` over my own `/tmp/siren-stems` to list stems (`read` on the directory lists it), and one parallel `read` that raced a `cp` it should have followed. I did not count the calls and will not give a number.
