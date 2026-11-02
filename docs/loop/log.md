# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Entries are **newest first**; iteration 0 is the repository's creation. Rules and traps are in
`AGENTS.md`; the design rules and their sources are in `../research/siren.md`; the fixed cases every iteration runs are
in [`cues.yaml`](cues.yaml); each iteration's own research, plan, evidence and three-pass review are in
[`runs/YYYY-MM-DD-NN/`](runs/README.md), and a question the loop will not decide itself goes to
[`needs-decision/`](needs-decision/README.md).

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

**Commit.** see below. Gate: build, test (41 + 4), serde (48 + 4), mixer (45 + 4), clippy, fmt, doc, examples all green
at the end; the mixer-example link failed once with `errno=28` while the disk was at 100 %, and passed after my own
throwaway crate was removed.

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
