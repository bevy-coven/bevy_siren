# Iteration 115 — plan: an Ogg loop is as long as its stream says it is, on both paths

## Ranked

Every open issue was re-read through `issue://`: `#32`–`#34` are the editor's (built in `bevy_grimoire`, which this loop does not patch; its pin is 221 commits back and its 14 GB target does not fit beside this crate's), `#35` waits on a person's anchor pairs, `#39` was decided not to ship (ADR 0005). **No `consumer` issue is open**, the baseline is green.

| # | item | rank | why it is where it is |
|---|---|---|---|
| **1** | **An Ogg loop is longer than its stream, on both paths** | **1. wrong output** | A 12 – 20 ms near-silent gap at every wrap (the threshold of normal hearing is ≈ 3 ms, `research.md` S6) and a bar drift of one padding a pass, in **the format Bevy enables by default**, on 28 % of the real files iteration 112 measured. The crate has known since 112 that `mixer` plays the padding (`decode.rs:14-16`, `semantic.md:160`) and cut it only where a model would notice. The route is found, the arithmetic is exact, and the default path needs **no decoder of its own**: `PlaybackSettings::duration` (S3). |
| 2 | A person listens to the mixer path | not mine | a machine cannot do it |
| 3 | A hubness correction measured with a held-out bank; the default-floor decision; resampler fidelity; a third library | experimental / the owner's | no consumer-visible effect today |
| 4 | `play` copies the whole sample (`bytes.to_vec()`) on every play; `Repeat` keeps the decoded loop in memory | 5. performance | real, bounded; the Bevy path does the second too |

## The design

**One page walk, in a core module** (`src/ogg.rs`, `pub(crate)`, always compiled — the default build needs it for Bevy's path, and it names no engine type and reads no file):

```rust
pub(crate) struct Stream { pub frames: u64, pub sample_rate: u32, pub channels: u16 }
pub(crate) fn inspect(bytes: &[u8]) -> Option<Stream>      // one Vorbis stream, pages unbroken to the end, last page eos, a granule the packet count allows
pub(crate) fn loop_length(bytes: &[u8]) -> Option<Duration> // inspect, then take_duration's arithmetic
```

`inspect` is `semantic::decode::ogg_declared_frames`'s guards (one bos, `offset == length`, eos on the last page, granule ≥ 0) **plus** the Vorbis identification header (`\x01vorbis`: channels, rate, block sizes) **plus the plausibility window** (`research.md`), because the playback paths cannot cross-check against a decoded
length as `semantic::decode` does. It returns `None` for anything it does not fully understand, and the caller then plays the file as it always did: **it can only refuse to trim, never trim wrongly on a stream it can see is odd.**

**`take_duration(frames, rate, channels)`** — `Duration::from_nanos(frames·channels·p + 1)`, `p = ⌊10⁹ / (rate·channels)⌋` (S4). It is a copy of a `rodio` implementation detail, so a test with the **real** `TakeDuration` pins it (default builds have no `rodio`; the pinning test is behind `mixer`).

**Bevy's path** (`src/playback.rs`, beside `start_pending`, in `SirenSystems::Play`): `trim_ogg_loops` — for each voice that is a loop, has an `AudioPlayer`, and has not been looked at, and whose asset is loaded, set `PlaybackSettings::duration = loop_length(bytes)` and mark it looked at (a private marker component). If the asset is not loaded, it asks again next frame. **It runs in `Update`; Bevy starts the sink in
`PostUpdate`**, so the duration is in place before the sink reads it (asset events are applied in `PreUpdate`). No public item, no message, no default, no schema.

**The mixer's path** (`src/mixer/mod.rs`): `play` computes `loop_length(bytes)` for a looping voice and `connect` gains `trim: Option<Duration>` — `looped(sample.take_duration(d), gain)` — so the chain is `decoder → take_duration → repeat → gain`. `once` is untouched: **only loops expose the padding** (a one-shot ends ≤ 20 ms late, inaudibly), and a one-shot's slot is freed by the sample's end either way.

**`semantic::decode`** keeps its behaviour and loses its private copy: `ogg_declared_frames` becomes `ogg::inspect` on the file's bytes, **plus the surplus cap it already applies** (it knows the decoded length). Every `semantic_decode` and `semantic_indexer` test is the regression check.

## The tests, in the order they are written (each fails first)

`src/ogg.rs` (unit, synthetic pages — the package ships `src/**` only, so no fixture is read here):
1. a well-formed stream (a 30-byte identification packet, comment and setup packets, `K` audio packets, a last page with the eos flag and granule `L`) → `Stream { frames: L, rate, channels }`;
2. **refusals**, one test each: two bos pages (chained or grouped); a last page without eos; a granule of −1; a file cut short; a first page that is not Vorbis (`OpusHead`); not Ogg at all; empty; **a granule the packet count cannot hold, one above the window and one below it**; the window's own edges are accepted.

`tests/ogg_loops.rs` (a module of `tests/all.rs`; real fixtures `tone.ogg`, `tone-441.ogg`, `tone.flac`, `tone.mp3`, a WAV):
3. a looping cue whose asset is `tone.ogg` gets `PlaybackSettings::duration = Some(_)` **in the frame it is played** when the asset is already loaded, and **in the frame the asset arrives** when it is not;
4. **none** for the same Ogg played once, for a looping FLAC, MP3 and WAV, and for a chained Ogg (`tone-441.ogg` twice) — the guards that stop it trimming what it does not understand;
5. behind `mixer`: through the **real** `rodio::Decoder` on the real bytes, `take_duration(duration)` yields **exactly `declared × channels` samples** for `tone.ogg` (48 kHz stereo, 24 000 frames: 48 000 samples where the untrimmed decode gives 49 152) and `tone-441.ogg` (44.1 kHz stereo, 4 416 frames), **and a loop of it recurs with exactly that period** — the arithmetic pinned against `rodio` itself.

`src/mixer/mod.rs` (unit, `SamplesBuffer`):
6. a sample with 576 near-silent frames of padding, looped through `connect` with the trim, has a **period of exactly `N`** and **no silent run at the wrap**; with `trim: None` it is unchanged (the control).

**Measured, not tested** (needs real Ogg files and a WAV writer): `evidence/after/ogg-loop.rs.txt`, a scratch program that `#[path]`-includes the crate's real `src/ogg.rs` and, with the real `rodio::Decoder`, reports §P1 – P6, renders four passes of `tone.ogg` **before and after through both pipelines**, and runs over Kenney's 254 Ogg files; then waveform pictures and spectrograms that are **read**.

**A `cues.yaml` case (35)**: *"a loop of an Ogg is as long as its stream declares, on both paths"*, with tests 3 – 6 as its evidence.

## Decisions recorded (nothing here needs the owner)

| decision | choice | why |
|---|---|---|
| the module's visibility | **`pub(crate)`** | a public item is API the owner keeps; every consumer of it is in this crate, and the behaviour is reached (and tested) through `PlaybackSettings` and the mixer |
| trim loops only, or every Ogg voice | **loops only** | only a loop exposes the padding; a one-shot would pay a page walk per play (a footstep played 50 times a second) for ≤ 20 ms of inaudible tail |
| the surplus cap on the playback paths | **replaced by the packet-count window** | the decoded length is not known without decoding; the window is the spec's own bound (`research.md`) and refuses, never trims |
| reading from a path or from bytes | **bytes** | both playback paths hold bytes; `semantic::decode` reads the file's bytes only when its first four are `OggS` |
| a positive starting granule | **named, not handled** | rare (a stream cut from a broadcast); the window refuses most; at worst the trim is *not shorter than the real length* |
| a public `needs-decision` file | **none** | no public item, message, default or schema changes |

## Carved off

- **Streaming**: `Repeat` still decodes its first span on the audio thread and keeps the decoded loop in memory; a long stem wants a streaming source. Not this iteration.
- **MP3 without a LAME/Xing tag** is not gapless; the tag is `symphonia`'s business and the crate does not read it.
- **Ogg Opus** is not a format this crate decodes (`rodio` is built with `vorbis`), so there is nothing to trim.

## What would change this plan

- If `tone.ogg` through Bevy's pipeline **does not** loop at 24 576 (P1), the premise is wrong for the default path and the item is withdrawn or reduced to the mixer.
- If `N·p + 1` is **not** exact in the real `TakeDuration` (P4), the route needs a different construction (a `Source` adaptor on the mixer's path and a documented off-by-`k` on Bevy's).
- If the plausibility window refuses real files (P8), it is widened or dropped, and the reason is written here.

## What happened to the plan

**P1 failed the way "What would change this plan" said it might, and the answer was not "withdraw".** `tone.ogg` through the crate's own pipeline did not loop at 24 576: it did not loop at all (0 of 147 456 samples). Reading
`rodio-0.22.2/src/decoder/symphonia.rs:163` and `source/buffered.rs:104` gave the mechanism (a fresh Vorbis decoder reports a span of length zero; `Buffered` reads that as an empty source); RustAudio/rodio#775 and Bevy's
0.18 → 0.19 migration guide (*"looping"*) say it is known, and the fix is on `master` and not in 0.22.2. The item became three changes, in this order of value:

1. **The loop sounds.** `looped` wraps its source in `OpenSpan` (`current_span_len().filter(> 0)`): a unit test over a source that starts with an empty span (red first), and a real-device test with a real Ogg (red without it, on this Mac).
2. **The crate stops selecting the decoder that breaks loops.** `Cargo.toml` names `lewton` for Vorbis — Bevy's own default — because `rodio` compiles `lewton` out when `symphonia-vorbis` is on, so the crate's line also swapped the decoder of a game's
   *own* audio. Decided by measurement: against `libvorbis` on 254 real files `lewton` is within 7 × 10⁻⁷ (`symphonia`: off by up to 0.30, clamping 25 hot masters), returns the declared length for 203 files (`symphonia`: 1), loops, and
   decodes 1.8× slower (23 against 41 million samples a second); **the Kenney golden benchmark with the real CLAP model is unchanged to four decimals on all seven metrics**. **No `needs-decision` file**: the directory's README reserves it
   for public API, messages and the schema, and this is internal and reversible in one line; the decision is in `Cargo.toml`, `CHANGELOG.md` and the log.
3. **The loop is the length the stream declares** — the original plan, unchanged (`src/ogg.rs`, `trim_ogg_loops`, `Shape::of`), now measurable end to end on a decoder that loops.

**What it cost.** A test that asserted `first == second` over a loop that was empty passed vacuously, so the end-to-end check of Bevy's chain held nothing until I compared a *length*, which is what made it fail.
`tests/ogg_loops.rs`'s rodio test now asserts `first.len() == trimmed` before the equality.
