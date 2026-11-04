# Iteration 114 — eval: the baseline, the cue table, and what the claims audit found

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`)

**Nothing is red.** Every command exits 0 and the counts are iteration 113's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 lib + **200** |
| `--features serde` | **209** |
| `--features semantic` | 109 lib + **517** |
| `--features mixer` | **210** |
| `--all-features` | 109 lib + **541** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build (*`ld: __eh_frame section too large (max 16MB)`* from `examples/mixer` against the full umbrella) is not this crate's code; iteration 112 recorded it.

## The cue table (`cues.yaml`)

All 33 cases are asserted by the matrix above (each case's `evidence:` names its tests) and none is red. **But the table has a hole that green hides**: case 15 (*"a layer change lands on the bar, and a silent layer stays in the mix"*)
observes `MusicState::target_db` — **a number the crate writes** — and case 8's and 26's mixer evidence is a fake `AudioOutput` that records the calls it receives. No case, and no test, observes **the samples the crate's own mixer
produces**. Every one of the 33 passes if `set_voice_db` is called and nothing audible follows from it.

## Half-done work

None. `git status` is clean at `f4f8dda` (= `origin/main`); the newest run directory with no verdict is this one. The open issues are as iteration 113 left them: `#32`–`#34` (the editor's, `bevy_grimoire`; none is filed
there with `consumer:bevy_siren`), `#35` (criterion 3 is a person's anchor judgements), `#39` (decided not to ship, ADR 0005; the owner has not closed it). **No `consumer` issue is open.**

## The claims audit — by `grep` and `read`, not by running anything

The skill's step 2 says a claim a test never exercises is a defect candidate. The sentence that is **repeated most often and held least**: *"Both answer `Played`, `Refused` and `Voice` identically; only the sample path differs"* (`README.md:83`,
`lib.rs:65`, `play.rs:7`, `output.rs`). The *sample path* is exactly what no test reads on the crate's own mixer, so I read it: `src/mixer/mod.rs::play` builds a looping voice as

```rust
let voiced = VoiceGain { source: decoder, gain: gain.clone() };
if looping { player.append(voiced.repeat_infinite()); } else { player.append(voiced); }
```

and `rodio-0.22.2/src/source/repeat.rs` builds `Repeat` by calling `input.buffered()` on whatever it is given: `Buffered` (`buffered.rs:98-125`) **pulls a span of up to 32 768 samples out of the source and stores the values**; every later pass replays
the stored values. Here the source is `VoiceGain`, so **the gain is multiplied in *before* the cache**.

**A probe, run before anything was changed** (`examples/scratch_loopgain.rs`, a scratch file, with `VoiceGain` copied from the crate verbatim and the real `rodio::Decoder` on `tests/fixtures/tone.flac`, one pass = 24 000 samples):

```
pass 1 rms 0.08837  (gain 1.00)
pass 2 rms 0.08837  (gain set to 0.25 before the pass: expected 0.02209)
pass 3 rms 0.08837  (gain set back to 1.00: expected 0.08837)
```

**A `Mode::Loop` voice on the crate's own mixer plays at the gain its first pass was decoded at, for ever.** Bevy's path is not affected: `bevy_audio-0.20.1/src/audio_output.rs:165` appends `decoder.repeat_infinite()` and the sink's volume is applied *outside* the repeat.

What that means for the crate's headline features, all on `--features mixer`:

- **Layered music.** A layer that starts at `SILENT_DB` (the crate's design: *"the layer at `SILENT_DB` stays in the mix"*) is decoded silent; moving it to 0 dB on the bar writes the atomic and changes nothing the player hears after its first pass.
- **A fade is a click.** `apply_stopping`'s 120 ms fade, a stolen loop's fade and a removed layer's fade are all `set_voice_db` ramps; the loop plays on at its old gain until `stop_voice` cuts it.
- **Within the first pass the gain moves in steps of up to 32 768 samples** (0.34 s of stereo at 48 kHz): `Buffered::extract` takes a whole span at the gain of the instant it is first needed.

## Ranked (`plan.md` has the full list)

1. **Bugs and wrong output — a loop on the mixer path ignores every gain change after its first pass.** Silent (nothing refuses, nothing logs), in the feature the crate is for, with an existing sentence claiming the opposite. Rank 1 by the skill's own rule.
2. *(carved off, measured, not built)* An Ogg loop is not sample-exact on either path: symphonia keeps the last block's padding (iteration 112: up to 975 frames), so each pass is longer than the stream declares.
3. Everything else in iteration 113's top five (a hubness measurement; the floor decision; resampler fidelity) is experimental or the owner's.

## After (`matrix-final.log`, the same script on the final tree)

Every command exits 0. The counts move only where tests were added: **six unit tests** in `src/mixer/mod.rs` (they run in the library's own test binary, so they appear with `--features mixer` and `--all-features`) and **one real-device test** in `tests/mixer.rs`.

| command | before | after |
|---|---|---|
| `cargo build` | ok | ok |
| `cargo test` | 6 doc + 200 | 6 doc + **200** |
| `--features serde` | 209 | **209** |
| `--features semantic` | 109 lib + 517 | 109 lib + **517** |
| `--features mixer` | 210 | **6 lib + 211** |
| `--all-features` | 109 lib + 541 | **115 lib + 542** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean | clean |
| `cargo fmt --check` | clean | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean | clean |

**Four matrix runs, one of them red** — `baseline-matrix.log` (arrival, green), `matrix.log` (after the mixer change, five unit tests, green), **`matrix-red-flake.log`** (the sixth unit test added, **red in two steps**) and `matrix-final.log` (green). The red run was **not this change**: two
indexer tests that both named their scratch directory `"formats"` raced for it (`evidence/before/indexer-scratch-race.txt`: ten of ten failures on two threads, no mixer code involved). It was a flake in the baseline since iteration 112, found by the fourth run; the six test helpers now give every
call its own directory (`evidence/after/indexer-scratch-race.txt`), and the final run is green.

**The cue table's hole is closed**: case 34 is the first case whose observation is *a sample the crate's own mixer produces*. Cases 15, 22 and 23 still observe numbers the crate writes (`target_db`, `ducked`, `layer_db`) — correct, and now agreed with by the sound on both backends.

**Still open after this iteration**: nothing from the baseline; the owner's decisions (`needs-decision/`), `#35`'s anchor pairs, the editor's `#32`–`#34`, `#39`'s closure. The carved-off Ogg item and its route are in `plan.md`.
