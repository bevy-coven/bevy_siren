# review.md — iteration 56, three passes

## Pass 1 — Function: does a dropped key stop holding its handle?

**The claim** (Böttcher 2008, the citation this module's own doc rests on): *"the main problem of procedural audio is not
the synthesis but the **management of thousands of assets**"*.

**Measured**, two tables of ten different keys:

| | keys held | what each one costs |
|---|---|---|
| before `retain_keys` | **20** | **one whole encoded audio file, resident** |
| after | **10** | the files the table names |

`Handle<AudioSource>` is 24 bytes and the map entry about 62 KiB per thousand — **the handles are the cost**, not the map.
`AudioSource` is `bytes: Arc<[u8]>`, the whole file, and a strong handle keeps it loaded.

**The test fails on the old tree** with `no method named retain_keys`, which is the honest shape for an additive API; the
merge itself is recorded by `a_second_table_leaves_the_first_tables_keys_held`, which uses only `len()` and runs on
**both** trees: twenty keys for two ten-key tables.

**Verdict: GREAT.**

## Pass 2 — Zoom: is a stale key ever read?

**This is the question that decides whether it is a bug or only untidiness, and it was answered by reading every read
site:**

- `missing()` iterates `table.sounds()` (`src/sounds.rs:101`) — the table's keys, never the map's.
- `failed()` likewise (`src/sounds.rs:114`).
- the only `sounds.get(...)` is `src/play.rs:538`, with `slot.sound` from the cue in the current table.

So a stale key **cannot** make a refusal lie, make `missing()` hide a delivery, or play the wrong file. It is purely
retention — which is why this iteration is ranked below the blocked report items and says so rather than dressing a
memory fix as a correctness fix.

**Three rank-1 hypotheses were probed first and found already correct**, and are kept as tests:
`a_stolen_voices_duck_recovers` (the duck reaches exactly `0.0` after the source bus empties),
`a_bus_level_changed_mid_voice_reaches_it` (`Music` 12 dB down within 0.5 s of the table being written), and the merge
test above.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: did anything audible move, and is the call order right?

**Nothing moved.** `render.txt`'s variant streams and the duck are byte-identical to iteration 55's:

```
variants footstep-grass: avoid 1, 3 sounds -> step_03 step_01 step_02 step_03 ...
bus-Music: duck -8.0 dB, deepest -7.96 dB, recovered by 1.97 s
mix.wav: -18.6 LUFS, -6.1 dBFS
```

A memory fix that changed a sample or a level would be a wrong fix; the spectrogram
(`evidence/spectrograms/grass-spectrum.png`) is a clean broadband transient at 0 decaying to a steady low band, peaks
below 0 dBFS.

**Call order, and why `load_table` was left alone.** `retain_keys(&new)` then `load_table(&new)`: dropping first means a
kept key is re-asked (the asset server has it cached) instead of a lost file being held across the load. `load_table`'s
own contract is *"returns the keys it asked for that it had no handle for yet"* — ask for what is missing — so removing a
key inside it would silently change what an existing caller's map means.

**Can a developer read it without the code?** Yes: the guide states the reload sequence and why, and `len()` now means
*the keys the table names*.

**Verdict: GREAT.**
