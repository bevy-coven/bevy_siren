# research.md — iteration 7: a voice slot is reused, not appended

## The item

`MixerBackend` keeps `voices: Vec<VoiceTrack>` per bus. `play` pushes and returns `len() - 1`; **nothing ever removes**.
`retire_voices` despawns the Bevy entity, but the backend's slot stays — holding the `rodio::Player`, its control
block and its queue. Probed: **500 one-shots with 0 live voices leave 500 distinct indices handed out and 500 retained
`VoiceTrack`s**. On a game's own mixer — the path whose whole point is a long-lived summing graph — that is unbounded
growth for the process's life, one entry per sound the game has ever played.

Iteration 2's log called this a leak and deferred it ("a memory leak of one `Player` handle per one-shot … not a
budget leak, and it is what a second iteration's 'compact the voice list' item is for"). It was never picked up. This
iteration picks it.

The budget bounds **concurrent** voices; this list counts **cumulative** ones. Both cannot be bounded by
`max_voices`: the first is a promise to the mix, the second is a promise to the allocator.

## Sources

1. **Gregory, *Game Engine Architecture* 3e, §14.5.8.4 "Instance Limiting"** (this crate's own citation): a sound
   engine "supports only a fixed number of simultaneous voices … so we must use them wisely". **Fixed** is the word the
   backend missed: it keeps one slot per sound ever played, so the "fixed number" grows without bound and the engine
   is using memory it was designed not to.

2. **The crate's own rule R4** (`docs/research/siren.md`): three hard caps, and *"a request that cannot be made room in
   is `Refused { VoiceLimit }` with nothing spawned"*. The budget is the crate's promise that voice count is bounded.
   The mixer's retained slots are the same resource counted a second time, and there the bound is absent.

3. **`rodio` 0.22.2, vendored.** `Player` holds `queue_tx: Arc<queue::SourcesQueueInput>`, `controls: Arc<Controls>`,
   `sound_count: Arc<AtomicUsize>` (`player.rs:20–28`). `Player::stop` sets a flag and does **not** free the queues;
   `Player::detach` consumes the sink "without stopping the sounds that are still playing". Dropping the `Player` is
   what releases the `Arc`s, and there is no `Drop` impl on `SourcesQueueInput` to do anything surprising on the way
   out — so **discarding the `VoiceTrack` is sufficient** and needs no explicit teardown call.

4. **`Output::set_voice_db` / `stop_voice` / `finished` take `usize`** (`src/output.rs`), and `MixerVoice(pub usize)`
   carries it (`src/play.rs:210`). An index is therefore a **stable name** that systems hold across frames:
   `update_mix` writes a gain by index, `apply_stopping` fades by index, `retire_voices` asks by index. Compaction that
   renumbers would silently address the wrong voice; that is the constraint the design answers, not an afterthought.

**Literature server unavailable, fifth iteration running.** `hs distill search` returned
`Embedding error: gpu busy: … < 5000 MB required to load bge-m3` on every attempt this iteration too (a 14.8 GB VLLM
engine holds the card; not this crate's process). No new paper is claimed; the rule is argued from Gregory's *fixed
number*, the crate's own R4, and the vendored `rodio` source, all of which this repository already holds.

## The technique

A slot is a **position plus a generation**, packed into the `usize` the trait already carries:

```
 63            32 31             0
+----------------+---------------+
|   generation   |     slot      |
+----------------+---------------+
```

- `play` finds a free slot (its `VoiceTrack` is `None` or its generation is stale), bumps that slot's generation, and
  returns `(generation << 32) | slot`. A slot that has never been used has generation 0, so the first return is
  `(1 << 32) | slot`.
- Every other method decodes `(slot, generation)`, and **does nothing when the generation does not match** — so a
  stale `MixerVoice` from an entity the world has not finished despawning cannot address the new occupant. That is the
  hazard, and the generation is the whole fix for it.
- Freed slots are found by scanning for a slot whose occupant has ended. The scan is over **slots**, which is bounded
  by the peak concurrent voice count plus whatever is still fading — not by sounds played — and it stops at the first
  free slot, so the common case is O(1).

Why not a `HashMap<u64, VoiceTrack>`? It would also bound growth, but it allocates on the audio-adjacent path and the
crate's rule is that the mixer's `next()` does not allocate; a `Vec` with a free-list scan keeps `play`'s cost a
linear scan of a small, bounded vector and no allocation at all after warm-up. Why not renumber? Because the index is
a name three systems hold across frames (source 4).

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| the bound on *concurrent* voices | `max_voices`, 32 by default | crate rule R4, unchanged |
| the bound on *retained* slots | the peak concurrent count (never more) | this design; **design default** |
| bits | 32 for the slot, 32 for the generation | `usize` on every supported target is ≥ 32 bits; a slot count above 2³² is not reachable by a voice budget |
| a generation's wrap | at 2³² plays of one slot | design default: unreachable in a session, and a wrap is a stale index whose occupant also ended |
| what frees a slot | dropping its `VoiceTrack` | vendored `rodio`: no `Drop` on the queue, `Player::stop` frees nothing by itself |

## What good sounds like

Nothing audible changes — this is the allocation side of the same mix. What changes is what a long session measures:

| after N one-shots, 0 live | before: retained slots | after |
|---|---|---|
| 8 | 8 | 1–2 (peak concurrent) |
| 500 | 500 | 1 (the fake reports every sample ended) |
| 10 000 | 10 000 | 1 |

A reviewer judges it with a count, not an ear: the probe's `distinct mixer voice indices handed out` stays at the peak
concurrent count however many plays run, and the *live voice* count is unchanged. The risk to judge is the opposite
one — a slot reused while an old entity still names it — and the generation is what makes that a no-op rather than a
crossed wire; the test for it is spelled out in `plan.md`.
