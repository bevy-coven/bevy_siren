# perf.md — frame cost at 8, 64 and 256 voices

`cargo run --release --example perfcheck`, this Mac (arm64, 8 cores, pinned 1.99.0). Load average **8.2 / 8.7 / 7.8**.

| voices | run 1 | run 2 | run 3 |
|---|---|---|---|
| 8 | 0.064 | 0.069 | 0.065 |
| 64 | 0.291 | 0.355 | 0.356 |
| 256 | 1.039 | 1.264 | 1.165 |

**The change cannot touch these numbers.** `perfcheck` calls `approach`, `db_to_gain` and `chain_db` over a fixed gain
list; it builds no `SirenSounds` and no cue table (`examples/perfcheck.rs:33-40`). This iteration added one method that a
project calls **on a reload**, not per frame — there is no new work in any frame.

**The 8-voice row is flat to 8 %** and the 256-voice row's spread is the host: iteration 51 measured a **2× spread on one
binary** at load 5.1.

## The cost this iteration removes, measured

Not a frame cost — a resident-memory one, which is the number that matters here:

| | value |
|---|---|
| `Handle<AudioSource>` | **24 bytes** |
| map entry, 1000 stale keys | **~62 KiB** |
| **each stale handle's real cost** | **one whole encoded audio file, resident** |
| keys held after two 10-key tables, before | **20** |
| after `retain_keys` | **10** |

The map overhead is negligible; the handles are not. `AudioSource` is `bytes: Arc<[u8]>` — the entire file — and a strong
handle in a live map is what keeps it loaded.

**What `perfcheck` measures.** The **crate's per-frame work as an upper bound**: every voice is costed, including the ones
`update_mix` skips because their gain did not move. It is not the audio thread.
