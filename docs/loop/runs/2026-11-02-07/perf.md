# perf.md — frame cost at 8, 64 and 256 voices

Measured with `cargo run --release --example perfcheck` on this Mac (arm64, 8 cores, toolchain 1.99.0).

**Machine state.** Load average **4.34 / 5.38 / 6.20**; disk 2.0 GB free, 100 % used.

| voices | µs/frame | µs/voice |
|---|---|---|
| 8 | 0.062 | 0.0078 |
| 64 | 0.354 | 0.0055 |
| 256 | 1.263 | 0.0049 |

**What this number is.** The crate's per-frame *game-thread* work driven by `model`'s pure functions — the shape of
`update_mix`, and an upper bound because the real system skips a voice whose gain did not move.

**This change is not on that path, and `perfcheck` cannot see it.** The work moved is inside `MixerBackend::play`, on
the audio-owning backend, once per sound started: a scan of a vector bounded by `max_voices` (32 by default) for the
first free slot, and no allocation beyond what `play` already did. A frame with no play request executes none of it.

The figure is higher than iteration 6's (0.059 / 0.329 / 0.827) because the machine's load is 4.3 rather than 6.2 at a
different moment and this Mac is shared; `perfcheck` drives neither the mixer backend nor the music systems, so the
comparison is the machine, not the commit.

## What *is* measured

The real backend, on this Mac's audio device, 400 one-shots of a 64-sample WAV:

```
before:  400 plays: distinct indices = 400, slots held = 400
after:   400 plays: distinct indices = 400, slots held = 6
```

**Slots bounded, names unbounded** — the pair that bounds memory without letting a stale name address a replacement.
`tests/mixer.rs::a_slot_is_reused_and_a_stale_index_addresses_nothing` asserts the same three facts headlessly where a
device exists, and skips with a sentence where it does not.
