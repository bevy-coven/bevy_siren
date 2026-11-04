# Iteration 114 — research: where a voice's gain sits relative to the repeat

## The question

`MixerBackend::play` builds a looping voice as `VoiceGain { source: decoder, gain }.repeat_infinite()`. **What does `rodio`'s `repeat_infinite` do to a gain that is applied inside it, and what does the crate's own rule say the gain must do?**

The probe in `eval.md` answered the first half before this note was written; what follows is why it is so, from the vendored source, and what the correct chain is.

## Sources

Every `rodio` and `bevy_audio` fact is read from the vendored registry source (`~/.cargo/registry/src/index.crates.io-*/`), as `AGENTS.md` requires; nothing here is from the web.

| # | source | what it establishes |
|---|---|---|
| S1 | `rodio-0.22.2/src/source/repeat.rs:10-19` (`repeat`), `:36-44` (`Repeat::next`) | `repeat_infinite` calls `input.buffered()` and keeps two **clones of one `Buffered`**. When `inner` ends, `inner = next.clone()`: **a pass after the first replays what the first pass stored**; the input is never asked for a sample again. |
| S2 | `rodio-0.22.2/src/source/buffered.rs:98-125` (`extract`), `:157-188` (`Buffered::next`) | A span is **pulled out of the input in one go** — `input.by_ref().take(min(span_len.unwrap_or(32768), 32768)).collect()` — and stored as `Vec<Item>`; `next` returns `data[position_in_span]`. So an adaptor *inside* the repeat is evaluated once per sample **of the first pass**, a whole span at a time, at the instant the span is first needed. |
| S3 | `rodio-0.22.2/src/player.rs:121-130` (`Player::append`) | The player wraps whatever it is given in `.speed(1.0) … .amplify(1.0) … .periodic_access(5 ms, …)`: **volume and speed are applied outside the appended source**, refreshed every 5 ms. |
| S4 | `bevy_audio-0.20.1/src/audio_output.rs:165` (`(None, None) => sink.append(decoder.repeat_infinite())`) | Bevy's own path repeats the **bare decoder**, and a sink's volume is S3's outside-the-source amplify. **The default path is the control: its loops are live.** |
| S5 | Khan et al. 2023, *Adaptive Background Music for a Fighting Game*, arXiv:2303.15734 (`docs/research/siren.md` §3, R5) | Adaptive music is a *"vertical layering technique, where the stems of a single piece are mixed in and out according to the game state"*: **per-instrument gain keyed to game state, the same tempo and beat throughout**. A stem is a loop, and its gain *is* the technique. |
| S6 | Gregory, *Game Engine Architecture* 3e, §14.5.8 and §14.6.3 (`docs/research/siren.md` §1, R2) | The mix is *"controlled by adjusting the level of each bus"*; a duck lowers a group for a while and **recovers**. A fade that does not fade is not a duck. |
| S7 | The crate's own contract: `src/mixer/mod.rs:11-13` (*"a voice is a `rodio::Player` … with its own gain as a second atomic"*), `src/mix.rs` (*"`update_mix` owns every sink and every mixer voice number"*), `AGENTS.md` (*"the two backends answer identically; only the sample path may differ"*) | What the code promises. A gain that is written and never heard is a **violation of R6** (*"everything a play decides is readable"*): `SirenMix` says the layer is at 0 dB while the speaker says nothing. |

## The technique, in the sources' terms

A voice's gain must be applied **at output time**, on every sample, whatever the sample's history. In `rodio`'s terms that is the order

```
Decoder  →  Repeat (stores the decoded, un-gained samples)  →  VoiceGain (reads the atomic per sample)  →  Player (speed, volume, 5 ms)  →  bus Mixer  →  BusGain  →  device
```

and the crate has the other order — `VoiceGain` *inside* `Repeat`, so the cache holds samples already multiplied by the first pass's gain. The fix is to swap two adaptors; the cost is nothing (one `Buffered::next` and one relaxed atomic load per sample, as before), and the cache then holds the
decoded sample once, at unity, which is what a cache of a *sample* should hold.

## Parameters the sources report

| value | where | what it means here |
|---|---|---|
| a span of **≤ 32 768 samples** | S2 `buffered.rs:112` | the granularity of a gain change *inside* the first pass under the old order: 0.68 s of mono, 0.34 s of stereo at 48 kHz |
| volume and speed refreshed every **5 ms** | S3 `player.rs:130` | Bevy's path (and `player.set_volume`) moves a loop's gain in 5 ms steps; the crate's atomic can do better — every sample |
| `SILENT_DB` = −80 dB → exactly `0.0` | `docs/research/siren.md` R6, `model.rs` | a layer "stays in the mix" at `0.0`; under the old order a layer decoded at `0.0` **replays zeros for ever** |

## What good sounds like

A looped tone whose gain is 1.0 on pass 1, 0.25 on pass 2 and ramps to 0 across pass 3 must show **three different envelopes** in a waveform picture: full height, a quarter of it, then a linear taper. Under the old order the three passes are identical. A spectrogram of the same signal shows the
tone's line at one level, then 12 dB lower, then fading — and no click at any pass boundary, because the loop is a whole number of cycles (220 cycles of 440 Hz in 0.5 s).

## Predictions (written before the change; the probe is the finding, these are what the fix should do)

| # | prediction | how it is checked |
|---|---|---|
| P1 | After the fix, through the real chain `Player → bus Mixer` pulled offline, **each pass's rms is the gain in force × pass 1's rms**: pass 2 / pass 1 = **0.250 ± 0.002** (the probe set the gain to 0.25) | a unit test and a rendered WAV |
| P2 | After the fix, **a gain stored between two samples is on the very next sample** (0 samples late). Before it, a store inside the first pass lands only at the next span boundary: **between 0 and 32 768 samples late**, and **never** after the wrap | stepping the gain at chosen sample indices and finding the first sample that carries it, for both orders |
| P3 | The loop chain's cost per sample is **unchanged within ±10 %** (both orders do one `Buffered::next` and one atomic load) | 10 s of audio pulled 20 times through each order |
| P4 | `Mode::Once` voices and Bevy's path are **unaffected** (`VoiceGain<Decoder>` is already live; S4) | the control test, and S4 by reading |
| P5 | **No existing test or cue case goes red**: the change is one function and its two call sites | the matrix |

## What this does not settle

- Whether a person **hears** a layer come in. Nobody has listened to the mixer path (iteration 112 said so); the render and the waveform are the evidence a machine can give, and the review says so.
- `Repeat` keeps the whole decoded loop in memory as `f32` (S2): a 3-minute stereo stem at 48 kHz is ~69 MB, ×N layers. That is `rodio`'s design and the Bevy path does the same; it is a cost to **document**, not this iteration's bug.
- **An Ogg loop is not sample-exact** on either path (symphonia keeps the last block's padding; iteration 112 measured up to 975 frames). Found while reading the loop code and **carved off**: `plan.md` records the route (`PlaybackSettings::duration` on Bevy's path — `audio_output.rs:160` builds `decoder.take_duration(duration).repeat_infinite()` — and `take_duration` on the mixer's), and the exact arithmetic `TakeDuration` needs, so the next iteration starts from it.
