# Iteration 114 — plan: a loop on the crate's own mixer ignores every gain change after its first pass

## Ranked

Every open issue was re-read through `issue://`: `#32`–`#34` are the editor's (built in `bevy_grimoire`, which this loop does not patch; its pin is 216 commits back and its 14 GB target does not fit beside this crate's), `#35` waits on a person's anchor
pairs, `#39` was decided not to ship (ADR 0005) and waits on the owner to close it. **No `consumer` issue is open**, the baseline is green, and the claims audit (`eval.md`) found one defect that outranks the backlog.

| # | item | rank | why it is where it is |
|---|---|---|---|
| **1** | **A `Mode::Loop` voice on the mixer backend plays at the gain its first pass was decoded at, for ever** | **1. wrong output, silent** | Probe: pass 2 rms 0.08837 where 0.02209 was commanded. Nothing refuses, nothing logs, `SirenMix` and `Applied` say the layer is at 0 dB while the speaker plays zeros. It is the crate's headline feature (layered music on the bar, S5) on its own backend, and the repeated sentence *"both backends answer identically"* says otherwise. |
| 2 | An Ogg loop is not sample-exact on either path (symphonia keeps the last block; up to 975 frames, 20 ms, per pass — iteration 112) | 1 or 2 | Real, measured, **audible above the ~3 ms gap threshold of normal hearing** (Brennan et al. 2018, `10.1097/aud.0000000000000558`, citing Irwin et al. 1981: ≈ 3 ms in broadband noise), and it drifts a stem off the bar by one padding per pass. **Carved off**: it needs a shared Ogg reader moved out of `semantic`, an exact-sample `Duration` for two different callers, and a Bevy-path system; item 1 is a one-function fix that does not wait for it. The route is below. |
| 3 | `play` copies the whole sample (`bytes.to_vec()`) on every play; `Repeat` keeps the whole decoded loop in RAM (a 3-minute stereo stem at 48 kHz is ~69 MB per layer) | 5. performance | Real, bounded, the Bevy path does the same for the second. A line in the guide, not an iteration. |
| 4 | The example's *"1 assets"* beside *"4 failed"* for a folder of broken files (iteration 112) | 3. ergonomics | Cosmetic. |
| 5 | A hubness correction with a held-out bank; the default-floor decision; resampler fidelity; a third library (iteration 113's list) | 5 / owner's | Experimental, or the owner's call. |

## The design

**One function and its two call sites** (`src/mixer/mod.rs`): the chain a voice plays is built in two private functions, so the test builds it the way `play` does.

```rust
/// A voice that plays once: the decoded sample, the gain applied to each sample as it is pulled.
fn once<S: Source>(sample: S, gain: Arc<AtomicU32>) -> VoiceGain<S> { VoiceGain { source: sample, gain } }

/// A looping voice: the sample **repeated, then** the gain — `Repeat` stores what it is given and replays it (`rodio` S1, S2), so a gain applied inside it is frozen at the first pass's value.
fn looped<S: Source>(sample: S, gain: Arc<AtomicU32>) -> VoiceGain<Repeat<S>> { VoiceGain { source: sample.repeat_infinite(), gain } }
```

`play` appends `once(decoder, gain)` or `looped(decoder, gain)`. Nothing else changes: no public item, no message type, no default, no schema, no dependency; `Mode::Once` voices and Bevy's path are untouched (**P4**). Per `AGENTS.md` this needs no decision file.

**Why not `Player::set_volume`** (the other way to make a loop live): it is a `Mutex` store refreshed every 5 ms (S3), where the crate's atomic is read per sample and already is the single writer's channel (`update_mix`). Using it would add a second path for the same number.

## The tests, in the order they are written (each fails on the unfixed order first)

`src/mixer/mod.rs`, `#[cfg(test)]` — private functions are tested where they live, as `src/semantic/*` does; a self-contained `rodio::buffer::SamplesBuffer` (the package ships `src/**` only, so a unit test must not read `tests/fixtures`):

1. `a_looping_voice_follows_its_gain_on_every_pass` — a 40 000-sample buffer of 1.0 (more than one 32 768-sample span, so a pass is two spans), looped, pulled across **three wraps** with the gain changed at the start of a pass and **in the middle of one**: every sample equals the gain in force when it was pulled. Fails on the old order at the first change after the wrap.
2. `a_gain_stored_between_two_samples_is_on_the_next_one` — the same on the first pass, where the old order is late by up to a span (**P2**).
3. `a_voice_that_plays_once_follows_its_gain` — the control (**P4**); passes on both orders.
4. `a_looping_voice_through_a_player_and_a_bus_mixer_follows_its_gain` — the same through a real `rodio::Player` and `rodio::mixer::mixer`, pulled offline: what the device would be handed, minus the device (**P1**).
5. **A `cues.yaml` case (34)** — *"a looping voice on the crate's own mixer follows its gain on every pass"*, with the test as its evidence. Cases 15, 22 and 23 observe `MusicState::target_db` and `SirenMix::ducked`, numbers the crate writes; none observed a sample the crate plays.

**Measured, not tested** (needs a decoder and a WAV writer): `evidence/after/loopgain.rs.txt` — a scratch program (deleted from `examples/` before the commit, its source kept as text) that renders, through the **real `rodio::Decoder` on `tests/fixtures/tone.flac`**, a `Player` and a bus mixer, three passes of a loop under a gain schedule (1.0 → 0.25 → a fade to 0) and a two-layer score (calm at 0 dB; combat at −80 dB entering at 0 dB on pass 2), once with each order, into `before/` and `after/` WAVs; then spectrograms and waveform pictures that are **read**; plus the per-sample cost of the two orders (**P3**).

## Decisions recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| fix by reordering, or by moving gain into `Player::set_volume` | **reorder** | one writer, one atomic, per-sample; `set_volume` is a 5 ms `Mutex` poll (S3) |
| expose the chain-building functions for tests/examples | **no, private** | a public item for a test is API the owner has to keep; the unit tests live beside the code, the evidence program carries its own copy |
| trim an Ogg's padding in the same change | **no** | a different bug with a different fix (a reader, an exact duration, a Bevy-path system); one build per iteration |
| a `Refusal`/`Entry` for a loop that ignored a gain | **no** | there is nothing left to report once it does not ignore it |

## Carved off, with the route (so iteration 115 starts from it)

**Ogg loops are not sample-exact.** Found while reading the loop code: `bevy_audio-0.20.1/src/audio_output.rs:145-166` already supports **`PlaybackSettings::duration`** — `decoder.take_duration(duration).repeat_infinite()` — so the *default* path can be made exact **without a custom decoder**, by setting the duration of an Ogg loop to the length its last page declares (the reader exists: `semantic::decode::ogg_declared_frames`, needing a bytes-based home outside `semantic`). The mixer path uses the same `take_duration` before `looped`. **The exact arithmetic**, read from `rodio-0.22.2/src/source/take.rs:101-126`: `TakeDuration` emits a sample while `remaining > p` and subtracts `p = ⌊10⁹ / (rate × channels)⌋` ns per sample, so it emits **exactly `N` samples when `remaining = N·p + 1 ns`** — not `N` frames divided by the rate, which is off by up to 25 samples at 4 s (the truncation of `p` is 6.4 × 10⁻⁵ relative at 48 kHz stereo). Vorbis I A.2 is the rule (*"a granule position on the final page … that indicates less audio data than the final packet would normally return … indicates how many trailing samples to discard"*, `xiph.org/vorbis/doc/Vorbis_I_spec.html`); a positive starting granule (A.2, the first-page rule) is **not** handled by the reader and must be named, not assumed away.

## What would change this plan

- If the offline `Player → Mixer` chain **does not** reproduce the probe (it should: the probe used the crate's `VoiceGain` verbatim and the real decoder), the unit tests above are wrong and the finding is the probe's alone — then the review says so and the item is withdrawn.
- If P3 shows the new order costs **more than 10 %** per sample, the fix still ships (it is a correctness bug) and `perf.md` says what it cost.
