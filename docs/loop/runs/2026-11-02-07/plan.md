# plan.md — iteration 7: a voice slot is reused, not appended

## The pick, and why it beats the runner-up

**Pick:** `MixerBackend` never frees a per-bus `VoiceTrack`, so a game on the crate's own mixer retains one `rodio`
player (and its queues) for **every sound it has ever played**. Probed: 500 one-shots, 0 live voices, 500 retained
slots. Iteration 2 named it and deferred it.

**Runner-up:** the three decision-blocked items (a cap in the refusal payload, an undecodable variant, removing
`Completes`). All wait on the owner; none is buildable today.

The pick wins on the loop's "costs little to run": an unbounded allocation per play is the one cost that gets worse
the longer a game is played, and it is invisible until a long session.

**Carved off:** a public `MixerVoice` handle type. The packed `usize` needs no public change; a type would be a break
for a gain that the existing signature already carries.

## Design

`src/mixer/mod.rs` only — the trait, `MixerVoice` and every system are untouched.

```rust
struct VoiceTrack {
    player: Player,
    gain: Arc<AtomicU32>,
    stopped: bool,
}

struct BusTrack {
    mixer: Mixer,
    gain: Arc<AtomicU32>,
    /// `None` is a free slot. The vector's length is the **peak concurrent** voice count, never the number of
    /// sounds played.
    slots: Vec<Option<VoiceTrack>>,
    /// The generation each slot is at. Bumped when a slot is handed out, so an index from a voice the world has not
    /// yet despawned cannot address its replacement.
    generations: Vec<u32>,
}
```

Encoding, in one place so every method agrees:

```rust
const SLOT_BITS: u32 = 32;
fn pack(slot: usize, generation: u32) -> usize { ((generation as usize) << SLOT_BITS) | slot }
fn unpack(voice: usize) -> (usize, u32) { (voice & ((1 << SLOT_BITS) - 1), (voice >> SLOT_BITS) as u32) }
```

- `play`: reuse the first free slot if one exists, else push; bump that slot's generation; return `pack`.
- `set_voice_db` / `stop_voice` / `finished`: `unpack`, and if the slot is free or its generation differs, **return
  without touching anything** — a stale name addresses nothing. `finished` returns `true` for a stale name, which is
  what a caller asking "has this ended" needs to hear about a voice that no longer exists.
- `retire` is not a trait method: the slot is freed lazily, by `play` finding `Some(track)` whose `player.empty()` or
  `stopped` is true and reusing it. That keeps the trait unchanged and the cost inside `play`, which already runs once
  per voice.

`voices(name)` (public, unused in this crate and the editor) keeps its meaning of "slots this bus holds" and now
reports the peak concurrent count; its doc says so.

## Every field touched on a serde type

None. `MixerBackend` and `BusTrack` are private; `MixerVoice(pub usize)` is unchanged; `AudioOutput`'s signatures are
unchanged.

## Tests that prove it

`tests/mixer.rs` (feature-gated, and the fake output does not exercise the *real* `MixerBackend`, so these tests drive
the real one through a headless device or assert on the encoded contract):

1. `a_refused_voice_index_is_a_slot_and_a_generation` — a pure test of `pack`/`unpack` round-tripping slot 0 and 7 at
   generations 1 and `u32::MAX`, so the encoding cannot drift.
2. `a_stale_voice_index_addresses_nothing` — set a voice at generation *n*, free the slot, hand it out again at *n+1*,
   then `set_voice_db`/`stop_voice`/`finished` with the old index: the new occupant's gain and player are untouched and
   `finished(old)` is `true`. **This is the hazard the generation exists for** and it must not regress.

`examples/budget.rs` — the probe that measured this becomes permanent evidence: after N plays with every sample
ended, report the **distinct indices handed out** beside the live voices, so an append-only backend is visible as a
line rising with N and a compacting one as a flat line at the peak.

No `CueTable`/`SirenSettings` behaviour changes, so `cues.yaml` gains one case (26) and no existing case moves.

## Verification

- the whole step-1 baseline and the `cues.yaml` table;
- `cargo run --release --example perfcheck` before/after at 8/64/256 — this one *does* touch the mixer's `play` path, so
  the number matters and must be reported honestly, including that `perfcheck` may not drive it;
- `examples/budget` before/after: `distinct indices` rising with N before, flat after;
- `examples/render` cue WAVs byte-identical.

## Risk

Reusing a slot while a stale `MixerVoice` is still held is the one way this can play a voice wrong, and it is why the
generation is not optional. The scan for a free slot is O(peak concurrent voices) worst case, which a budget of 32
bounds; `play` already allocates a `Player` per call, so the scan is not the cost that matters. The one behaviour a
consumer could notice: `MixerBackend::voices()` now reports the peak concurrent count rather than the played count —
it is unused in this crate and in the editor, and the doc says which it is.
