# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Entries are **newest first**; iteration 0 is the repository's creation. Rules and traps are in
`AGENTS.md`; the design rules and their sources are in `../research/siren.md`; the fixed cases every iteration runs are
in [`cues.yaml`](cues.yaml); each iteration's own research, plan, evidence and three-pass review are in
[`runs/YYYY-MM-DD-NN/`](runs/README.md), and a question the loop will not decide itself goes to
[`needs-decision/`](needs-decision/README.md).

## Iteration 6 — 2026-11-02 — a refused sample names the sound, not the cue

**Change.**

- **`src/playback.rs` — a decode failure names the sound key.** `start_pending` reported
  `MissingSound { sound: voice.cue }` — the cue's name in a field documented as the key a project maps to a file — so a
  corrupt `step_03.wav` sent a person looking for `footstep.grass`. It now recovers the key from the same cue and
  variant `answer` chose (`key_of`), and reads `Res<CueTable>` to do it. No public type, field or message changed.
- **`src/play.rs` — one sentence for the variant's two causes:** *"no handle or undecodable bytes for sound
  `step_03`"*, with the doc saying which two call sites produce it.
- `examples/budget.rs` — a `missing-sound.txt` probe that runs both causes side by side in the real ECS;
  `tests/mixer.rs` — a `refuses_samples` mode on the fake output (the same `None` the real mixer returns when
  `rodio::Decoder::new` errors) and two tests; `tests/play.rs` — the sentence; `cues.yaml` case 25;
  `needs-decision/undecodable-sound.md`; the cues guide, the CHANGELOG and a media note.

**Why.** Top of iteration 5's next-5, and its own class: a refusal confidently naming the wrong subject, in the crate
whose refusals exist to name what is missing. Found by iteration 5's probe, which fed four zero bytes to the mixer and
got `no handle for sound \`line\`` for a cue whose key was also `line` — a coincidence that hid it from every test.

**Evidence.** `runs/2026-11-02-06/`: `eval.md` (baseline green; the table's gap named — case 11 covers only the
missing-handle cause), `research.md`, `plan.md`, `perf.md`, `review.md`, and `evidence/{before,after}/` with both
render trees, the two-cause probe and the cue spectrograms.

**Measured.**

| cause | before | after |
|---|---|---|
| a key with no handle | `sound: "step_03"` (true) | `sound: "step_03"` |
| a key whose bytes do not decode | **`sound: "footstep.grass"`** (the cue) | `sound: "step_03"` |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.059 / 0.329 / 0.827 µs | not on this path; the machine's load was 6.2 against 2.7 in iteration 5, which is the whole difference |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect in my own work: a third test
tried to resolve a default handle's id so a waiting voice would reach the code under test, and `Assets::insert` on that
id does not make `assets.get` succeed — the test proved nothing on any tree and was removed rather than contorted.

**Traps for the command.** (1) A default `Handle::<AudioSource>` **never** resolves, however you insert into `Assets`
afterwards: a test that wants "bytes arrive later" must build a real handle and hold *that*. (2) A cue and its key
having the same name (`line`/`line`, as in `examples/budget`) hides this whole class of bug — the probe's first run
looked correct for exactly that reason. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM
engine holds the card), so the rule is argued from the vendored `bevy_asset`/`rodio` sources and the crate's own R6.

**Commit.** `409eac6`, pushed to `main`; CI run 41553622842 is **success** on `big-s1`. Gate: build, test (53 + 4), serde (60 + 4), mixer (59 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; this change's host effect is a corrected `Entry::Refused` sentence, which the panel prints verbatim.

**Next 5.**

1. **A variant that distinguishes "never mapped" from "corrupt file"** — `needs-decision/undecodable-sound.md`. A new
   `match` arm for a consumer, so not decided here; option 4 in that file (log the cause) is one line and needs no
   answer.
2. **A payload that also names the cap** — `needs-decision/refusal-names-its-cap.md`.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **The mixer's decode error is discarded** at `mixer/mod.rs:147` (`Decoder::new(..).ok()?`); carrying it into the
   refusal would tell a project *why* the file failed, not only which one. Wants the same decision as item 1.
5. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.

**Baseline note.** Nothing was red, and the suite had 52 tests. Case 11 proves the missing-handle cause; no case could
reach the decode cause through `SirenSounds` alone, which is why iteration 5's probe found it by accident.

---

## Iteration 5 — 2026-11-02 — a refusal names the bus only when the bus refused

**Change.**

- **`src/play.rs` — the refusal names the cap that refused.** `room_for` has three hard caps and returned one
  uninformative `None` from all of them, so `answer` always blamed the cue's bus. It now returns
  `Result<Vec<Entity>, Cap>` with a private `Cap::{Cue, Bus, Table}`; `bus: Some(name)` only for the bus's own cap,
  `bus: None` otherwise — which is what the field's own doc says (*"the bus that had no room, when one did"*).
  No public type, field or message changed.
- `examples/budget.rs` — a **`refusals.txt` probe** that forces each of the three caps in the real ECS and writes the
  payload and the sentence; a real WAV per cue, because dummy bytes are refused by the decoder (`MissingSound`) before
  the budget is ever consulted.
- `tests/mix.rs` — four tests: table and cue caps name no bus (both fail on the unfixed tree), a full bus still names
  it (passes on both, so the fix cannot overshoot), and each payload shape has exactly one sentence.
- `examples/render.rs` — the three `VoiceLimit` sentences; `needs-decision/refusal-names-its-cap.md`; `cues.yaml` case
  24; the buses guide, the CHANGELOG (with the host consequence) and a media note.

**Why.** The loop's first criterion, and worse than a silent failure: a **confidently wrong** one. The probe showed a
table-budget refusal accusing `Ambience` for a cue on `Effects` — a bus with a free slot, not even the cue's own.
Cases 7–9 each asserted `matches!(reason, VoiceLimit { .. })`; the wildcard was the hole, and no case read the payload.

**Evidence.** `runs/2026-11-02-05/`: `eval.md` (baseline green; the table's gap named), `research.md`, `plan.md`,
`perf.md`, `review.md`, and `evidence/{before,after}/` with the two render trees, the refusal texts and the budget
probe.

**Measured.**

| cap that refused | before | after |
|---|---|---|
| the table's `max_voices: 1`, two buses with room | `Some("Ambience")` → ``no voice free on bus `Ambience` `` | `None` → `no voice free` |
| the cue's `max_instances: 1`, its bus at 0 of 8 | `Some("Effects")` | `None` → `no voice free` |
| the bus's `max_voices: 1` | `Some("Effects")` | `Some("Effects")` — unchanged, and now the only one |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | — | 0.022 / 0.096 / 0.667 µs — `room_for` runs per request and only its refusal changed; the harness does not drive it |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found one defect **in my own evidence
harness** (four zero bytes as a sample, so the decoder refused before the budget did — every line read
`MissingSound { sound: "line" }`, and the harness was measuring the wrong subsystem), and it exposed a second real
refusal bug, logged rather than folded in.

**A number worth keeping.** The before tree named a bus in **every** refusal, and the first row names a bus the cue
does not play on. "No voice free" is less informative but true; the diagnostic version needs the cap in the payload,
which is the decision file.

**Traps for the command.** (1) `examples/render` **hardcodes** its refusal strings, so it cannot show a change to a
refusal — the evidence for this iteration is `examples/budget`'s ECS probe, and `render`'s identical refusal files are
expected, not a failure. (2) An app that inserts a dummy `AudioSource` measures `start_pending`'s decoder, not the
budget: use a real WAV. (3) `hs distill` is still GPU-blocked (`bge-m3` wants 5000 MB; a 14.8 GB VLLM engine holds the
card), and the metadata API returned no abstract for the instance-limiting papers, so the rule is argued from the
crate's own R4 and Gregory §14.5.8.4's *per-group limits*, which this repository already cites.

**Commit.** `3d158ae`, pushed to `main`; CI run 41552664607 is **success** on `big-s1`. Gate: build, test (52 + 4), serde (59 + 4), mixer (56 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"`; it was not run, and this change's host-visible effect is a different `Entry::Refused` sentence —
the panel prints it verbatim, so it improves without a consumer change.

**Next 5.**

1. **`start_pending` reports a decode failure as `MissingSound { sound: <cue> }`** — a cue's name in a field documented
   as the sound *key*. A project greps its asset list for `footstep.grass` and finds nothing. Found by this
   iteration's probe.
2. **A payload that names the cap** — `needs-decision/refusal-names-its-cap.md`, with the three sentences option 2
   would print.
3. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`.
4. **A crossfade a second change interrupts** replaces target and fade rather than queueing; a design question the log
   names.
5. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.

**Baseline note.** Nothing was red, and the suite had 48 tests. The defect was invisible because every test that
reaches a `VoiceLimit` matched it with `..`: the kind was asserted, the content never.

---

## Iteration 4 — 2026-11-02 — a crossfade takes the time it was given

**Change.**

- **`src/music.rs`, `src/model.rs` — `MusicLayers::fade_s` is honoured.** It was stored in `Pending` and read by
  nothing; `move_layers` eased every layer at `MIX_TAU_S`, so an authored 800 ms crossfade arrived in ~150 ms.
  `model::tau_for_fade` derives the constant from `approach`'s own definition — `fade_s / ln(1000)`, an arrival within
  one part in a thousand — and each `LiveLayer` carries the fade its own change gave it.
- **`move_layers` holds a layer's gain on a frame with no elapsed time.** `approach` returns its target for `dt <= 0`,
  which is right for a bus on a game's first frame and wrong for a layer: the frame that applied a change could spend
  the whole fade in one step. The tests found this; it is fixed in the crate, not in the tests.
- `examples/render.rs` — a **layer-fade plot** (the same move under `MIX_TAU_S` and under an authored 0.8 s) and its
  caption in `render.txt`; `tests/model.rs` and `tests/music.rs` — four tests, one of which fails on the unfixed tree;
  `cues.yaml` case 23; the music guide, the CHANGELOG and a media note.

**Why.** The top of iteration 3's next-5, and the same class as its two predecessors: a number a person wrote whose
sentence the code did not honour, silently. Rank 2 in the loop's order (music transitions and fades) but rank 1 in
practice — the field is public, documented and already in every project's builder chain.

**Evidence.** `runs/2026-11-02-04/`: `eval.md` (baseline green; the one red case is the fade), `research.md`,
`plan.md`, `perf.md`, `review.md`, and `evidence/` with both render trees, the fade plot and its zoom, and the cue
spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| 0.3 s into an authored 0.8 s fade | **−9.13 dB** (settled) | **−13.6 dB** (arriving) |
| the same fade at 0.85 s | −9.0 dB | −9.07 dB → its target |
| `fade_s(0)` at 0.5 s | −9.0 dB | −9.0 dB (default easing unchanged) |
| a layer the change does not name | — | keeps its own fade |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.019 / 0.103 / 0.684 µs | the same — the harness does not drive the music systems |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot**: a
fixed `0 dB` vertical scale that pinned a fade from silence to the floor, a marker labelled "duck fires" on a plot
with no duck, and a t=0 marker drawn on the y axis where its label collided. All fixed before the verdict.

**A number worth keeping.** The derivation is not a fitted constant: `approach` reaches `1 − e^(−t/τ)`, so "arrive
within a thousandth at `fade_s`" is `τ = fade_s / ln(1000)`, and `tests/model.rs` asserts both halves of that —
it has arrived at `fade_s`, and it has **not** arrived at `fade_s / 2`.

**Traps for the command.** (1) A headless test's **first** update has `dt = 0` unless `advance()` ran first, so a test
that writes a message and immediately updates tests the step branch of `approach`, not the fade. Every existing music
test advanced first; the new ones must too. (2) `perfcheck` cannot see a change on the music path — say so in
`perf.md` rather than presenting identical numbers as a measurement. (3) An evidence harness that draws *both* the old
and the new curve prints the same line whichever tree it was built from; that identity is by design and is not
evidence of the fix — the test is.

**Commit.** `951e627`, pushed to `main`; CI run 41551366531 is **success** on `big-s1`. Gate: build, test (48 + 4), serde (55 + 4), mixer (52 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test still pins
`rev = "7029b20"` and would compile the crate's third predecessor, so it was not run; no public type, field or message
moved.

**Next 5.**

1. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing a `Refusal` payload wants a decision file;
   the log line could be fixed without one, and probably should be.
2. **`Completes` is public and can never be true** — `needs-decision/remove-completes.md`. One line to remove in the
   next minor.
3. **A crossfade that a second change interrupts** replaces the target and the fade rather than queueing. Whether a
   game wants queueing, and from which gain it should start, is a design question the log now names.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release; shipped at `DEFAULT_FADE_S`.
5. **`Rng::next_f32` deliberately discards the low bits** of a xorshift64\* output (the weak ones). A sentence in
   `model.rs` stops the next reader re-deriving it.

**Baseline note.** Nothing was red, and the suite had 44 tests; the fade defect was invisible to it because no test
ever wrote a non-zero `fade_s` and then read the layer before it settled.

---

## Iteration 3 — 2026-11-02 — the seed a project writes, and the hold a stinger promises

**Change.**

- **`src/play.rs`, `src/lib.rs` — `SirenSettings::seed` is the stream.** `SirenChoices` was built once from a crate
  constant and never rebuilt, so a project's `seed:` reached nothing: `seed: 7` and `seed: 99` produced
  byte-identical `Played` streams. A new `play::seed_choices` system in `SirenSystems::Choose` replaces the stream —
  and every variant history with it — when the mix table's seed changes. It runs before the music starts a layer, so
  a score started on the same frame draws from the new stream too.
- **`src/music.rs` — a stinger's `duck_s` is a hold.** `play_stingers` passed it as `DuckClock::manual`'s *fade*
  argument with a hold of zero, so the `Music` bus began recovering on the frame the stinger fired. It now holds for
  `duck_s` and releases over `DEFAULT_FADE_S` (0.12 s), the release every stop and every steal already uses.
- `examples/render.rs` — a **bus-gain plot** beside the stinger envelope, with both readings of `duck_s` in one run;
  `tests/play.rs` and `tests/music.rs` — five tests, two of which fail on the unfixed tree;
  `docs/loop/needs-decision/stinger-release.md`; `cues.yaml` case 17 rewritten and case 22 added; the cues and music
  guides, the CHANGELOG and a README line.

**Why.** Both are the loop's first-ranked class — wrong output, silent, from a number a person wrote whose sentence
said something else. Case 17 existed to prove reproducibility and could not, because a project could not choose a
stream; the stinger's `duck_s` was documented as a hold and passed as a fade, so the music swelled back underneath the
punctuation mark. Neither needed a new model: `active_duck_db` already had the shape, and the seed field already
existed.

**Evidence.** `runs/2026-11-02-03/`: `eval.md` (baseline green; the four next-5 items checked against the code — one of
iteration 2's was **wrong**, case 9 *does* have tests), `research.md`, `plan.md`, `perf.md`, `review.md`, and
`evidence/` with both render trees, the stinger envelope plot and its zoom, and the cue spectrograms.

**Measured.**

| measurement | before | after |
|---|---|---|
| `seed: 7` vs `seed: 99`, 8 plays | **identical** (variant, gain, pitch) | different from the first play |
| `seed: 7` twice, two processes | identical | identical |
| `Music` halfway through a 0.6 s, −9 dB stinger | **−5.01 dB** | **−8.99 dB** |
| every rendered cue and the mix | — | **byte-identical** (16/16 WAVs) |
| frame cost, 8 / 64 / 256 voices | 0.075 / 0.326 / 1.090 µs | 0.023 / 0.135 / 0.508 µs — the machine's load fell 4× between runs; neither fix is on this path |

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found three defects **in my own plot** (no
legend; the caption reporting the axis top as the peak; the "duck fires" label on the axis, crossing the release
curve) and one in my reading of it — the blue shelf looked like a slope at 900 px until the emitted polyline was
checked frame by frame: frames 39–48 are exactly −9.00 dB. All fixed; the zoom file shows the shelf as a shelf.

**Traps for the command.** (1) An evidence harness that hard-codes the *fixed* shape cannot show the old one — print
**both readings** of a changed argument in one run and let the numbers disagree, rather than rendering "before" from a
reverted tree that the harness does not read. (2) A plot's y-scale must include the cap/depth or a flat shelf sits on
the axis and reads as a curve; check the polyline's y values, not the thumbnail. (3) A `find ~ -iname …` for a paper
takes minutes on this disk — the corpus is on `big`, not here. (4) The home-still embedder was GPU-blocked for all 23
attempts of this iteration (`gpu busy: 4222 MB free < 5000 MB required`); `research.md` records that rather than
pretending a source was read.

**Commit.** `f4fca21`, pushed to `main`; CI run 41549988083 is **success** on `big-s1`. Gate: build, test (44 + 4), serde (51 + 4), mixer (48 + 4),
doctests, clippy, fmt, rustdoc, examples and the README quick start, all green. The Cauldron panel test was not run:
`bevy_cauldron` still pins `bevy_siren` at `rev = "7029b20"`, so it would compile this iteration's *predecessor* and
prove nothing; no public type, field or message moved this time either.

**Next 5.**

1. **`MusicLayers::fade_s` is stored in `Pending` and never read.** Every layer move eases at `MIX_TAU_S` (50 ms), so
   an authored 800 ms crossfade is silently 50 ms. Internal only — carry the fade into `LiveLayer`, or set the easing
   time constant from it — so no decision file is needed.
2. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit; the message shape change wants a decision file.
3. **`needs-decision/remove-completes.md`** — the public `Completes` component cannot ever be true. One line to remove
   it in the next minor, if the owner agrees.
4. **`needs-decision/stinger-release.md`** — a project-chosen stinger release, shipped this iteration at the crate's
   `DEFAULT_FADE_S`. The field is additive; the default keeps behaviour identical.
5. **`Rng::next_f32` uses the top 24 bits of a xorshift64\* output.** Fine for a variant pick; worth a look before
   anyone uses `SirenChoices::rng` for anything wider, because the low bits of xorshift64\* are the weak ones and this
   implementation discards them deliberately — a sentence in `model.rs` would stop the next reader re-deriving it.

**Baseline note.** Nothing was red at the start, and the suite had 44 tests: the two fixed defects were both invisible
to it, one because no test had ever written a seed and the other because the stinger test read the duck only on the
frame it fired.

---

## Iteration 2 — 2026-11-02 — a voice that ends frees its slot, on both backends

**Change.**

- **`src/mix.rs` — `retire_voices` asks the backend that owns playback.** On the `mixer` backend there is no
  `AudioSink`, and nothing ever set `Completes`, so a `Mode::Once` voice was never retired: it kept its place in the
  voice budget for the life of the process. The system now asks `AudioOutput::finished` — the query the trait already
  declared and `MixerBackend` already implemented, with **zero callers before this commit**.
- **`src/mix.rs` — `update_mix` writes a `MixerVoice`'s own gain.** It used to `continue` past a mixer voice entirely,
  so `Applied` was never written: a music layer moved on the bar stayed at its start gain, and a stop fade began at
  silence and despawned the voice on its first frame — a cut where the crate promises a 120 ms fade. One writer still
  owns both numbers; Bevy's sink gets the whole chain, the mixer's voice stage gets the voice's own gain.
- `examples/budget.rs` (`--features mixer`) — the evidence harness: N one-shots against a cap, a scripted output whose
  voices end after a set life, writing per-frame CSV, a text summary and an SVG plot.
- `tests/mixer.rs` — four tests behind the feature, one of which fails on the unfixed tree.
- `docs/loop/cues.yaml` cases 20 and 21, `docs/guide/buses.md` (both promises made exact),
  `docs/loop/needs-decision/remove-completes.md`, a `CHANGELOG` "Fixed" pair, and a README line.

**Why.** This is the loop's most valuable class of bug — a refusal that should not happen, silent, and in the
backend whose whole point is that a bus is one number. Gregory §14.5.8.4 (*Instance Limiting*): a sound engine
"supports only a fixed number of simultaneous voices … so we must use them wisely", and a voice that stops *"frees
its resources for use by other voices"*. A voice that has produced its last sample and keeps its slot is exactly that
leak. The second defect fell out of the first test: the mixer voice's gain was never written at all.

**Evidence.** `runs/2026-11-02-02/`: `eval.md` (baseline green and why that was the problem; the case table, with
cases 9 and 17 still red), `research.md` (Gregory's lines, `rodio`'s `Player::empty`/`MixerSource`), `plan.md`,
`perf.md`, `review.md`, and `evidence/` with 46 WAVs/text from `examples/render` and four budget runs.

**Measured.**

| scenario | before | after |
|---|---|---|
| 40 one-shots, cap 32, stealable | 40 played, peak **40** live, **0** `ended`, last frame 32 live | 40 played, peak **3**, **40** `ended`, last frame 0 live |
| 6 one-shots, cap 4, priority 180 (unstealable) | **4** played, **2** `VoiceLimit`, **0** `ended`, 4 live forever | **6** played, **0** refused, **6** `ended` |
| a layer moved to −3 dB on the mixer | `Applied` never written; layer stayed at its start gain | `Applied { db: -3.0 }` |
| rendered audio | — | `render.txt` and all 17 WAVs **byte-identical**; cue −24.4 LUFS / −12.1 dBFS, `mix.wav` −4.3 LUFS / +0.2 dBFS |
| frame cost, 8 / 64 / 256 voices | 0.062 / 0.353 / 1.396 µs | 0.065 / 0.332 / 1.233 µs (inside the noise; `perfcheck` does not drive the changed code) |

**The stealable row is the one that matters.** With a stealable cue the leak is *invisible*: every play is answered
`Played`, because the permanently-full budget steals each time — 32 footsteps, then each new one cuts off a footstep
still sounding. The lockout row is what makes it a refusal a reader can see.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT. The zoom pass found four defects **in my own plot** (a
missing legend entry drawn outside the viewBox, the caption reporting the axis top as the peak, a cap label colliding
with the axis, and a blank raster because CSS variables do not resolve in a rasteriser); all four were fixed before
the verdict.

**Traps for the command.** (1) An evidence plot whose legend is anchored from the right edge silently loses an entry
when the two labels together overflow; anchor from the left and check the `x` extents. (2) A `--features mixer` test
that wants a started voice needs a **resolved** `AudioSource` handle, not `Handle::default()`: `start_pending` gives
a `MixerVoice` only once the bytes exist, so a default handle tests the loading state instead. (3) `errno=28` from
`ld` on this Mac is a full disk, not a code error — `/tmp` held 6.6 GB of my own probe build.

**Commit.** `6a62195`, pushed to `main`; CI run 41545567270 is **success** on `big-s1`. Gate: build, test (41 + 4),
serde (48 + 4), mixer (45 + 4), clippy, fmt, doc, examples, and the README quick start, all green. The mixer-example
link failed once with `errno=28` while the disk was at 100 % — the loop's documented trap, not a code error — and
passed after my own throwaway crate and the editor's build directory were removed.

**The Cauldron was not re-verified, and cannot be by this run.** `bevy_cauldron` pins `bevy_siren` at
`rev = "7029b20"`, so `audio_panel_reads_the_crates_file` compiles the *old* crate: it would prove nothing about this
change. It was attempted anyway and died of `No space left on device` during the 8.3 GB build. The public API did not
move (no type, field or message changed), so the panel's four reads — `SirenMix::db`, `ducked`, `voices_on`, the
`SirenLog` tail — keep their shapes; what changes for the panel is the number in `voices_on`, which is the fix. Its
next pin should move to this commit.

**Next 5.**

1. **Case 9 has no test.** An `UNSTEALABLE_PRIORITY` voice that must refuse rather than be stolen is the rule's other
   half, and the budget's evidence now shows a refusal path worth pinning. It needs a full bus and a dialogue cue.
2. **Case 17 pins the RNG, not the stream.** A determinism test over a whole `Played` sequence (two identical runs,
   equal field for field) is the case's own observation.
3. **`MusicLayers::fade_s` and `Stinger::duck_s` are stored and never read.** A crossfade a project asks for happens
   at `MIX_TAU_S` (50 ms) instead; the field's default is a lie about the shape of the fade.
4. **A `VoiceLimit` refusal names the bus even when the table's budget was the cap that failed** (`answer` always
   passes `bus: Some(bus)`). A panel prints the wrong culprit. Changing the message shape wants a decision file.
5. **`Completes` is public and can never be true.** `needs-decision/remove-completes.md` has the options; the fix
   landed without it, so it is safe to remove in the next minor.

**Baseline note.** Nothing was red at the start. The suite was green with the `mixer` bug present because no test gave a
voice a `MixerVoice` and then let its sample end — which is why the first two items above are about tests rather than
features.

---

## Iteration 1 — 2026-11-02 — the loop's eyes: a render harness and a case table

**Change.** The loop now has a way to judge audio it cannot hear, and none of it touches `src/`:

- `examples/render.rs` — renders every cue variant, every refusal, a ducked bus envelope, a mix and 8/64/256 voices to
  22.05 kHz mono WAVs plus a `render.txt` of numbers, with no `AudioPlugin` and no `SirenPlugin`: it drives
  `bevy_siren::model`, where every decision's arithmetic lives.
- `examples/perfcheck.rs` — µs per frame and per voice at 8, 64 and 256 voices, an upper bound on `update_mix`.
- `docs/loop/cues.yaml` — 19 fixed cases, each naming its behaviour, its observation and its green condition, with the
  source for every number. The case table is the contract a run is judged against.
- `docs/loop/` — `log.md` (moved from `docs/loop-log.md`), `runs/README.md` and `needs-decision/README.md` describing
  the shape of a run and of a question the loop will not decide itself.
- `.omp/skills/build-loop/SKILL.md` — the loop, written to this brief as a **skill** (the model reads it with
  `skill://build-loop`), with this machine's measured facts in it.

**Why.** The loop's brief demands spectrograms, loudness numbers and panel screenshots before anything ships. The crate
had a panel and a panel is not an ear; nothing could render a cue to a file that `ffmpeg` could measure.

**Evidence.** `runs/2026-11-02-01/`: `eval.md` (the baseline, the case table, loudness), `research.md` (BS.1770-4,
Gregory, the panner), `plan.md`, `perf.md`, `review.md`, and `evidence/` with 23 WAVs and text, four spectrograms and
waveforms, and the duck's gain plot as a line.

**Measured.** 256 voices cost 1.253 µs a frame — 0.0075 % of a 60 fps frame. A cue at −6 dB authors −6.00 dB and reads
−24.4 LUFS (looped past the 0.4 s gating block) at −12.1 dBFS true peak. The duck leaves 0 dB at 1.00 s, reaches
−7.96 dB, and recovers by 1.97 s against a 0.25 s hold and a 0.6 s fade.

**Verdicts.** Function GREAT, Zoom GREAT, Fresh eyes GREAT — with one caveat written down: `voices-8.wav` peaks at
3.419 because the harness sums without the voice budget, which is case 19's and the budget's question.

**Traps recorded in the command.** A bus gain is a step function between frames, so one sample per frame is a 4 ms
file; `active_duck_db` takes the time *since the duck fired*, not the score's clock, or the duck never recovers;
`showwavespic` on a gain curve draws an unreadable blob, so a bus plot is a line; and `ebur128` reads −70 LUFS for any
clip shorter than one gating block, whatever its level.

**Commit.** `4e99c64`. Gate: build, test (41 + 4), serde (48 + 4), mixer (41 + 4), clippy, fmt, doc, examples — all
green; nothing was red at the baseline.

**Next 5.**

1. **Case 19:** teach `examples/render` to model the voice budget, or move the case into `perf.md`, so a mix's evidence
   says how many voices it holds.
2. `tests/cues.rs` automating cases 1–4 and 17, so `cues.yaml` and the suite cannot drift.
3. The Cauldron's audio panel should surface the last refusal — a refused audition is a log line today, not a panel row.
4. Occlusion and a low-pass on the `mixer` backend (backlog, iteration 0).
5. Loudness targets per bus: BS.1770-4 is the measure, the target is a `needs-decision/` question.

---

## Iteration 0 — 2026-11-02 — repository setup

**What shipped.** The crate, whole:

- `src/model.rs` — engine-free maths: dB ↔ gain with one silence floor, a linear sum for simultaneous sounds and a
  dB chain for gains in series (a silent stage silences the chain), semitone ratios with a clamp, three distance
  laws and the backend panner's own law divided back out, a duck envelope, an exponential approach, the
  next musical boundary from tempo, a seeded xorshift, a no-repeat variant picker, and voice stealing by lowest
  priority then oldest.
- `src/cue.rs` — `CueTable`, `Cue`, `SoundSlot`, `PitchRange`, `Repeat`, `Mode`, `Attenuation`, and `problems()` in
  sentences. `from_ron`/`to_ron` behind `serde`.
- `src/mix.rs` — `SirenSettings`, `BusLevel`, `Duck`, `DuckClock`, `SirenMix`, `Stopping`, `Applied`,
  `UNSTEALABLE_PRIORITY`; the systems `update_mix`, `apply_stopping`, `retire_voices`, `stop_requests`.
- `src/play.rs` — `Play` → exactly one `Played` or `Refused`; `Voice`, `Compensation`, `Samples`, `MixerVoice`,
  `Completes`, `StopVoice`, `StopCue`, `SirenChoices`; the three caps checked in the order a person asks them.
- `src/playback.rs` — `start_pending`: the one place a voice starts on the crate's own mixer once its bytes load.
- `src/music.rs` — `MusicScore`, `MusicLayer`, `MusicLayers` with `When::{Now, NextBeat, NextBar}`, `Stinger`,
  `MusicState`; layers are voices started through `Play`, so the music has no second spawn path.
- `src/sounds.rs`, `src/log.rs`, `src/output.rs`, `src/mixer/mod.rs` — the key map, the 256-entry log, the
  `AudioOutput` trait and its `rodio` summing graph.
- `src/lib.rs` — `SirenPlugin`, `MusicPlugin`, `SirenPlugins`, `SirenSystems::{Choose, Play, Mix}`.

**Verified.**

| Check | Result |
|---|---|
| `cargo build` (the trimmed feature list a consumer gets) | green |
| `cargo test` | 49 tests, green |
| `cargo test --features serde` | 49+ tests, green |
| `cargo test --features mixer` | 49 tests, green |
| `cargo test --all-features --doc` | green |
| `cargo test --features readme-example --test all readme::` | green (README quick start == crate-root doctest) |
| `cargo build --examples` | green |
| `cargo clippy --all-targets --all-features -- -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | green |
| `cargo run --example music` | transitions at t = 0.00, 4.80 and 7.20 s — every one a multiple of the 2.4 s bar, `transitions on the bar: yes` |

**Captures.** `docs/media/mixer.png` and `docs/media/mixer.gif`, from `examples/mixer` at
`SIREN_FRAMES=240 SIREN_KEYS=steal,layer,duck SIREN_KEY_EVERY=25`, release build, `mean_ms 21.3` (240 frames). The
still reads: Music −9.6 dB with `duck −8.0` and 2 of 12 voices, Effects −3.8 dB with `duck −4.0` and 1 of 2, the cue
table at rest, and a log whose ten lines are the dialogue play, its two duck lines, a steal and its `stolen by`
line. **The captures earned their keep three times**: the first showed a missing glyph (an em dash is not in the
bundled font) and the cue list printed three times (one `Kind::Cues` shared by three nodes); the second showed a
bus at two of two whose steal had already expired; the third showed the music layer at −80 dB because the layer's
level and its cue's trim were both −4 dB. Each was a defect in the example, not in the crate, and each was fixed
before the image shipped.

**One real defect the tests then found.** The capture's steal step asked for three voices in one frame, and all
three played on a bus capped at two: the budget took its snapshot of live voices before the loop, so the frame's
own requests could not see each other. `play_requests` now keeps an owned `Live` list per request, drops what it
stopped and pushes what it started, and
`tests/mix.rs::three_requests_in_one_frame_fill_a_cap_of_two_with_no_third` pins it.

**Cauldron wiring (S-01).** `bevy_grimoire`'s `bevy_cauldron` hosts this crate behind an optional `bevy_siren`
feature, landed as `aea2456` and merged in `d6c6579`, pinned at `7029b20`; `src/audio_panel.rs` reads `<project>/family/bevy_siren.ron` into the crate's own
`SirenSettings` and `CueTable`, draws the bus rows from `SirenMix` and the log tail from `SirenLog`, and auditions the
selected cue with `Play::cue`. `bevy_siren#1` (S-01) and `bevy_grimoire#274` are the two records the two-way street
asks for, both filed in this pass.

Verified on the merged editor tree: `cargo test -p bevy_cauldron --features bevy_siren --test editor` → 1144 passed,
4 failed, and the four are `a_person_finds_and_runs_a_command_in_the_palette`, which fail identically with the
feature off (checked: 0 passed, 4 failed) — they predate this work. `cargo clippy -p bevy_cauldron --features
bevy_siren --all-targets` is clean, and `boot_headless` opens the shipped project on both feature sets: 12 frames,
2048 entities, no panic. The four panel tests read `Played`/`Refused` through a reader system: draining the doubled
message buffer after several updates lost one in the full parallel run, and a host reads every frame, so the test
does too.

**Decided (assumed, owner may override).**

- **Two backends, not one.** `bevy_audio` by default and our own `rodio` graph behind `mixer`, because a bus's gain
  as one number per bus per frame is the point of a bus, and Bevy's path cannot express it — but Bevy's path is the
  one that pans a moving emitter, so it stays the default.
- **The panner's distance law is divided back out.** rodio's `Spatial` multiplies each ear by `min(1, 1/d²)` as well
  as panning, so the crate's own attenuation would be applied twice. `model::backend_spatial_gain` mirrors that
  formula; `tests/model.rs::the_backend_spatial_gain_is_the_panner_distance_law` pins it.
- **`UNSTEALABLE_PRIORITY` is 180.** Music (200), stingers (210) and dialogue (180) keep their voices when effects
  fill the budget. A number, not a flag, so a project can position a cue between them.
- **The default duck is Gregory's pair.** Dialogue lowers music 8 dB and effects 4 dB, hold 0.25 s, fade 0.6 / 0.4 s.
- **`rust-toolchain.toml` pins 1.99.0.** `bevy 0.20` and this crate require 1.97.1; the machine's `stable` was 1.94,
  which cargo refuses. 1.99.0 is installed here and CI's `dtolnay/rust-toolchain@stable` is newer still.

**Backlog, ranked.**

1. **Occlusion and a low-pass on the `mixer` backend.** A wall between the listener and a sound should muffle it, and
   the mixer has no filter in its graph today. Needs a rodio filter source per voice and a ray-cast hook the game
   supplies — the crate must not own physics.
2. **A dialogue queue with subtitles.** `Dialogue` voices are unstealable but unordered: two lines at once play over
   each other. A queue with a `DialogueLine { speaker, text, cue }` and an accessibility subtitle stream is what a
   game actually needs next.
3. **Per-voice reverb zones.** A zone is data (a box and a wet mix); a voice inside it gets a send. The mixer path can
   do this properly; Bevy's path cannot, which is a reason for the `mixer` feature to grow.
4. **`SirenProbe`.** A way to ask "what would this cue play, and at what gain" without playing it, so the Cauldron's
   panel can preview a decision on hover instead of on click. Filed here as an `S-NN` when the editor needs it.
5. **A moving emitter's pan on the `mixer` backend.** rodio's `SpatialPlayer` can be re-positioned; a mixer-owned voice
   cannot today, which is the one thing Bevy's path does that ours does not.

**Release: published.** `cargo publish` from `big` (whose `~/.cargo/credentials.toml` carries the `[registry]` token;
this Mac has none, and the first attempt here was refused for that reason) uploaded `bevy_siren 0.1.0`. The registry
answers 200 for `https://crates.io/api/v1/crates/bevy_siren` with `max_version 0.1.0` and `Ladvien` as owner, and the
tag `v0.1.0` is pushed at `8e14231`. `docs.rs/bevy_siren/0.1.0/bevy_siren/` answers 200 — docs.rs had built it within a
few minutes of the upload, and the bare crate URL lags that by one more sync. The README's own image URLs answer 200.

**CI.** Two things were wrong, and neither was the code. Every sibling repo has a self-hosted runner registered and
this new one had none, so GitHub had nowhere to run; the runner is now deployed (`bevy-coven/bevy_siren 1` in
`home-cloud/gh-runners/repos.txt`, `deploy.sh`, `big-s1 online`). And `run: cargo test … readme::` is **not valid
YAML** — unquoted, `readme::` parses as a mapping and GitHub rejects the whole workflow, which is why every run still
failed in 0 seconds with no job even with a runner present. Quoted, it is a string. Run 41527805041 is the first real
one: **both jobs green on `big-s1`, 4m29s** (`fmt + clippy + doc`, then `build (standalone) + test`).
