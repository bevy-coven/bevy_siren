# bevy_siren — the loop log

One entry per iteration: what shipped, what was measured, the captures, and the ranked backlog the next iteration
starts from. Rules and traps are in `AGENTS.md`; the design rules and their sources are in `docs/research/siren.md`.

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

**Release: refused, recorded.** `cargo publish --dry-run` packages and verifies the 0.1.0 package (18 files, 82 KiB
compressed) — but `cargo publish` stops at `error: no token found, please run cargo login`: there is no
`~/.cargo/credentials.toml` on this machine and `CARGO_REGISTRY_TOKEN` is unset. The 0.1.0 commit stands; the tag is
unwritten and the next pass retries after a `cargo login`. Nothing was created on the owner's behalf.

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
