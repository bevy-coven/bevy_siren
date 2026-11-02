# plan.md — iteration 2: retire an ended voice on the crate's own mixer

## The pick, and why it beats the runner-up

**Pick:** a `Mode::Once` voice on the `mixer` backend never retires, so after `max_voices` one-shots (32 by default)
every play is refused `VoiceLimit` forever. Fix: `retire_voices` asks `AudioOutput::finished` for a mixer voice, the
query the trait already declares and the backend already implements.

**Runner-up:** `MusicLayers::fade_s` and `Stinger::duck_s` are accepted, stored in `Pending`, and never read — a
crossfade a project asks for silently happens at `MIX_TAU_S` (50 ms) instead. That is a real ergonomics defect (rank
3), and it is the next item: it changes the shape of a fade, not whether a sound plays at all.

The budget bug wins on the ranking's first criterion twice over: **a refusal that should not happen** and **a
message dropped** (the ended line), and it is silent — the exact class the brief calls the most valuable bug.

**Carve:** the item is "a voice frees its slot when it ends", both backends. It is *not* "a virtual-voice list",
"minimum play time" or "distance-based stealing" — all real in Gregory, none needed to make the budget release. Those
are carved off and recorded in the log's next-5.

## Design

`src/mix.rs`:

- `retire_voices` gains `output: Option<ResMut<Output>>`. For each voice with `Some(MixerVoice(i))` and not
  `Stopping` and not `Mode::Loop`, ask `output.0.finished(&voice.bus, i)`; a `true` retires the voice with the same
  `Entry::Stopped { why: "ended" }` the Bevy path writes.
- The Bevy path is unchanged: `sink.is_some_and(AudioSinkPlayback::empty)`.
- One system, both backends, one log line. No new public type, no new field, no new number.

`src/mixer/mod.rs`:

- `AudioOutput::finished` already returns `track.voices.get(voice).is_none_or(|v| v.stopped || v.player.empty())`.
  It is correct; it simply had no caller. **No change needed** — verified by the test, not assumed.
- `stop_voice` and the retired slot: a voice that ended is despawned by `retire_voices`, so the mixer's own
  `VoiceTrack` is never asked to stop it. The `MixerBackend` keeps its `voices: Vec<VoiceTrack>` entries forever
  (a slot index is stable once handed out). That is a memory leak of one `Player` handle per one-shot, *not* a
  budget leak, and it is what a second iteration's "compact the voice list" item is for. It is named in the log.

## Every field touched

None. This iteration adds **no field** to `CueTable`, `SirenSettings` or `MusicScore`, and no new refusal.

## Tests that prove it

`tests/mixer.rs` (new file, `#[cfg(feature = "mixer")]`-gated in `tests/all.rs`):

1. `a_one_shot_voice_ends_and_frees_its_slot` — with a fake `AudioOutput` that reports a voice finished, play 40
   one-shots against `max_voices: 32`; assert 40 `Played`, no `VoiceLimit` refusal, at most one live voice, and 40
   `Stopped { why: "ended" }` lines. **This is the failing-before/passing-after regression test.**
2. `a_loop_voice_is_not_retired_when_its_source_ends` — a `Mode::Loop` voice whose mixer voice reports finished stays
   live: a loop has no end to notice, and `rodio`'s `repeat_infinite` should never report one; the crate must keep
   the rule whichever way the source behaves.
3. `a_stopping_voice_is_not_retired_twice` — a voice already fading is skipped, so a steal's fade is never cut short
   by the ended check (its `Stopped` line says `stolen by`, not `ended`).

The fake `AudioOutput` is the point: a headless test cannot decode a WAV into a device, but it *can* implement the
trait the crate already talks to. No `AudioPlugin`, no device, no `unwrap`.

`tests/all.rs`: add the module under the feature so `every_test_file_is_a_module` stays true and the file is not
compiled without `mixer`.

## Verification

- the whole step-1 baseline, plus `cargo test --features mixer`;
- `cargo run --release --example perfcheck`, before/after (a change must not cost an untargeted case more than 10 %);
- `cargo run --release --example render` before/after and the spectrograms/loudness for the evidence directory;
- the Cauldron's panel test in `~/bevy_grimoire` (the crate's public API does not move, but the panel reads
  `SirenMix::voices`, and that number is exactly what this changes).

## Risk

`retire_voices` now queries `Option<ResMut<Output>>` on every frame in every build. With the feature off it is
`None` and the branch is one `is_none` check per voice; with it on, one virtual call per once-voice per frame. The
perfcheck harness measures the crate's `update_mix` cost, which is untouched, so that number is expected to be flat;
the retire loop's own cost is measured in the new test's 40-voice run and reported in `perf.md` as a wall-clock
mean.
