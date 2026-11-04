# bevy_siren — notes for agents

Sound for Bevy 0.20: a cue table as data, buses that duck, a voice budget that refuses politely, layered music that
changes on the bar, and a log of what played and why. Its design rules and their sources are in
`docs/research/siren.md`.

README and docs follow https://github.com/bevy-coven/.github/blob/main/DOCUMENTATION.md; a feature is not done until its README line and, if visible, its GIF exist.

## Source of truth

This repository, [`bevy-coven/bevy_siren`](https://github.com/bevy-coven/bevy_siren), is canonical. MIT OR Apache-2.0. A
change a game or the editor needs is made **here**, in its own commit with its own test, and requested here as an issue
labelled `consumer` and `consumer:<project>`; it is never patched into the consumer.

## Build and test

```sh
cargo build                                   # the trimmed feature list a consumer gets
cargo test                                    # one `all` binary, including tests/leaf.rs
cargo test --features serde                   # the RON round trips
cargo test --features semantic                # retrieval, the tool host, and `siren-tools` as a process
cargo test --features mixer                   # the mixer backend's own code paths
cargo test --doc                              # nextest skips doctests; this does not
cargo test --features readme-example --test all readme::
cargo build --examples
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
```

**`cargo build` is the load-bearing step**, not `cargo test`: the dev-dependency is the full `bevy` umbrella (the
examples need a window), and Cargo unifies features across a build — so under `cargo test` the crate silently receives
every Bevy feature and a hole in its own `[dependencies]` list cannot fail.

## Layout

- `src/model.rs` — every number the crate decides with and **no engine type**: `db_to_gain`, `gain_to_db`, `sum_db`,
  `chain_db`, `semitones_to_ratio`, `distance_gain`, `backend_spatial_gain`, `active_duck_db`, `approach`,
  `next_boundary`, `Rng`, `VariantPicker`, `Candidate`, `steal`. `tests/all.rs::engine_free` scans this file for engine
  names.
- `src/cue.rs` — `CueTable`, `Cue`, `SoundSlot`, `PitchRange`, `Repeat`, `Mode`, `Attenuation`, `CueProblem`,
  `problems()`, `from_ron`/`to_ron`.
- `src/mix.rs` — `SirenSettings`, `BusLevel`, `Duck`, `DuckClock`, `SirenMix`, `Stopping`, `Applied`,
  `UNSTEALABLE_PRIORITY`, and the systems `update_mix`, `apply_stopping`, `retire_voices`, `stop_requests`.
- `src/play.rs` — `Play`, `Played`, `Refused`, `Refusal`, `Voice`, `Compensation`, `MixerVoice`, `Completes`,
  `Samples`, `StopVoice`, `StopCue`, `SirenChoices`, and `play_requests`.
- `src/playback.rs` — `start_pending`: the only place a voice starts on the crate's own mixer once its bytes have
  loaded.
- `src/output.rs` — `AudioOutput` (the trait a backend implements), `Output` (the boxed resource), `Backend` (which
  path is live).
- `src/mixer/mod.rs` — behind `#[cfg(feature = "mixer")]`: one `rodio::Mixer` per bus, a `BusGain` and a `VoiceGain`
  source reading an `AtomicU32`, and `MixerBackend` implementing `AudioOutput`.
- `src/music.rs` — `MusicScore`, `MusicLayer`, `MusicLayers`, `When`, `Stinger`, `MusicState`, `MusicVoice`, and the
  systems `start_layers`, `adopt_layers`, `move_layers`, `play_stingers`.
- `src/sounds.rs` — `SirenSounds`: the key → handle map, `load_table`, `missing`, `failed`.
- `src/log.rs` — `SirenLog` (256 entries) and `Entry`.
- `src/semantic/server.rs` — behind `semantic`: the MCP tool host (`Host`, `SPECS`, `read_message`) as a library, so a test
  drives it with no process. `src/bin/siren-tools.rs` is the loop around it (ADR 0006, 0008); `serde_json` is its one
  dependency.
- `examples/semantic.rs` and `tools/export_clap_onnx.py` — the one place the **real** CLAP graphs run: the script makes
  them from a Hugging Face cache and checks itself against `transformers`; the example indexes a folder, ranks a
  description and plays the answer. CI has no model, so **no test runs them** (clippy compiles the example) —
  `docs/guide/semantic.md`, "Try it", is how to run them, and `docs/media/README.md` regenerates the picture.
- `src/lib.rs` — the plugins, `SirenSystems::{Choose, Play, Mix}`, the re-exports, the quick start doctest.

## The loop, as a skill

`.omp/skills/build-loop/SKILL.md` is this project's build loop — one iteration: orient, pick the most valuable item,
research it, build within the crate's rules, test, judge by spectrogram and loudness, three GREAT reviews, ship, log.
Read it with `skill://build-loop` before an iteration. It is a **skill** rather than a slash command because it is the
model's procedure, not a keystroke: `.omp/skills/<name>/SKILL.md` is what the native provider discovers at startup, and
one name has one home. It also carries the four traps that have already cost an iteration each (a bus gain is a step
function between frames; a duck takes time since it fired; a gain curve is a line, not a waveform; `ebur128` reports
−70 LUFS for any clip under its 0.4 s gating block). Add a trap to it when one costs you a pass.

## Rules

- **`src/model.rs` names no engine type.** It is the half that can be tested with no app, and `tests/all.rs` enforces
  it. A `Vec3` there is a bug even when it compiles.
- **Every play answers exactly one of `Played`/`Refused`.** A refusal names what is missing; nothing plays on a refusal.
- **One writer per voice gain.** `update_mix` owns every sink and every mixer voice number; nothing else writes a
  volume. `MusicLayers` moves a *target*, never a sink.
- **The two backends answer identically.** A test must pass with and without `--features mixer`; only the sample path
  may differ. A voice carries `Samples` always, an `AudioPlayer` only when the mixer does not own playback, and a
  `MixerVoice` only once the mixer started it.
- **`dB <= SILENT_DB` is silence**, on the way in and out. Never compare a linear gain to an epsilon.
- **Real time, not the game clock**, for the mix and the music: a menu that pauses the game must not freeze a fade.
- **No `unwrap()`/`expect()` on caller data** in `src/`. A table that cannot parse is a `String`; a distance that is
  `NaN` is at the listener's feet.
- **No game vocabulary in `src/`.** `tests/leaf.rs` forbids a game, the editor and a physics engine by name, and the
  dependency list is closed at `ALLOWED_DEPS` in that file: adding a name takes the sentence that argues for it, in the
  same commit (the file says so, and so does the build loop).

## Traps paid for

- **`AudioPlugin` opens a device**, and adding it twice panics. `SirenPlugin` never adds it; a headless test adds
  `MinimalPlugins + AssetPlugin` and calls `init_asset::<AudioSource>()`.
- **Without an audio device, an `AudioPlayer` entity still exists** and never gets a sink. That is what lets a headless
  test assert on `Played` and `Voice`, and why `retire_voices` must not treat "no sink" as "ended".
- **`Time<Real>` in a headless app only moves on `TimeUpdateStrategy`.** `Time::advance_by` is overwritten by the next
  `TimePlugin` update; `tests/helpers.rs` inserts `ManualDuration` instead.
- **`rodio`'s `Spatial` pan multiplies by `1/d²`.** Applying a cue's own attenuation on top would apply two distance
  laws; `model::backend_spatial_gain` is the panner's law, divided back out.
- **A `Mode::Loop` voice has no end to notice.** `retire_voices` skips loops; they live until stopped.
- **A rodio source cannot be re-positioned inside a `Player`.** That is why the `mixer` path applies spatial
  attenuation at play time, and why Bevy's path stays the default for moving emitters.

## The two-way street, both ways

**A consumer that needs something files it here, and this loop builds it here.** A game or the Cauldron opens an issue
on `bevy-coven/bevy_siren` labelled `enhancement` (or `bug`), `consumer` and `consumer:<project>`, titled `S-NN:
<what>` with the next free `NN` after the highest `S-` title, whose body says why, the proposed API, the edge cases,
and an acceptance a test in this repository can prove. An open `consumer` issue is taken right after bug issues and
ahead of the owner's focus; the fix is made here, in its own commit with its own test, as the general form (a project
is the example, never a name in the API), and the issue is closed only by that commit's `Fixes #N`.

**This crate's own needs go the other way.** When a cue, a bus rule, a music feature or a field of `SirenSettings`
cannot do what the Cauldron's audio panel needs, the request is filed on `bevy-coven/bevy_grimoire` labelled
`enhancement`, `consumer` and `consumer:bevy_siren`, and the editor item waits with `Needs: bevy_grimoire#N`. Never
patch, wrap or vendor the editor here.

Both directions carry the "recommend, record, proceed" rule and the `assumed-decision` label: an unanswered design
question is decided, recorded and built, never a reason to idle.

## Force pushes and deletions need the user's express permission

Never do any of the following unless the user has approved that specific action, by name, in the current conversation.
A loop brief, a skill, a standing instruction such as "commit, push, PR, merge", or an earlier session's approval is
not permission.

- Force-push in any form: `git push --force`, `--force-with-lease`, `--force-if-includes`, a `+` refspec, or pushing
  rewritten history. Deleting a remote branch or tag (`git push --delete`, `gh pr merge --delete-branch`, an API
  delete).
- Throwing away commits or uncommitted work: `git reset --hard`, `--merge` or `--keep`; `git checkout -- <path>` or
  `git restore` over changes you did not make; `git clean`; `git stash drop` or `clear`; `git branch -D`; `git worktree
  remove --force`; `git rebase` or `git commit --amend` on pushed commits; `git filter-repo` or `filter-branch`.
- Deleting files, directories, build caches, target dirs, models, datasets, database rows, issues, PRs, releases or
  cloud resources that you did not create in this session (`rm`, `rm -rf`, `git rm`, `cargo clean`, an API delete).

When one of these seems necessary, stop and ask. Name the exact command, what it destroys and whose work it touches. A
commit that landed on the wrong branch is fixed with a new commit or a question, never by resetting a shared working
tree or force-pushing. Files you created yourself in this session (scratch scripts, temp dirs, your own worktrees) you
may delete.
