# Iteration 118 — perf

What changed in a hot path: `play_requests` picks a `Decode` once a frame (`output.as_deref().filter(|_| backend.0)` and a `map_or`) and, **on the crate's own mixer only**, asks the backend about a sound's bytes — once per buffer, then from a memo. Nothing in `update_mix`, the mixer's audio thread (`next()`), or the render path changed.

## The check, first sight (release, a real device, mean of 300 runs after 20 warm-ups; `evidence/after/decoder-agreement-and-cost.txt`)

A sound on the mixer path now costs one decoder construction **in addition to** the one `play` makes, once per sound:

| file | `decode_error` | `play` (+ stop) | Bevy's builder |
|---|---:|---:|---:|
| Ogg Vorbis (5.8 kB) | 658 µs | 650 µs | 635 µs |
| Ogg Vorbis (4.5 kB) | 634 µs | 633 µs | 629 µs |
| MP3 | 10.5 µs | 11.2 µs | 10.4 µs |
| FLAC | 21.9 µs | 22.8 µs | 21.5 µs |
| WAV (s24, f32, µ-law, A-law) | 16 / 21 / 7.0 / 7.3 µs | 17 / 22 / 7.7 / 8.1 µs | 16 / 21 / 7.0 / 7.3 µs |
| refused: ADPCM WAV / Opus / LFS pointer / empty / cut / one byte | 3.1 / 3.6 / 1.1 / 0.8 / 0.9 / 0.7 µs | the same | the same |

The decoder construction dominates `play` (the `to_vec()` copy of a few kB and the `Player` are within the noise of it), so the check roughly doubles a Vorbis's first-play decoder cost: **+0.63 – 0.66 ms, once per sound**, and nothing for a file refused (the voice is never built). A game that plays a hundred distinct Oggs pays 65 ms in total, spread over however many frames it first plays them in.

## The memoised hit (a temporary `#[ignore]` unit test in `src/sounds.rs`, removed before the commit; `cargo test --release --lib`, 5 000 000 calls each)

| asker | per call |
|---|---:|
| the mixer's (`Decode::Output`) | **20.7 ns** |
| Bevy's (`Decode::Bevy`) | 24.5 ns |
| `Decode::Skip` | 5.1 ns |

Iteration 117 measured Bevy's hit at 21 ns; the same function with one more `match` arm reads 24.5 ns here, inside what this machine's runs differ by. The mixer's hit is a lock, a `HashMap` lookup and a 16-byte key: a play on the mixer path pays 21 ns it did not pay before, against the 1 – 10 µs the rest of a play costs (`play_requests` was not re-timed as a whole; this is the added work and no more).

## `perfcheck` (release, three runs, `update_mix`'s cost per frame; **not touched by this change**)

| run | 8 voices | 64 voices | 256 voices |
|---|---:|---:|---:|
| 1 | 0.018 µs | 0.112 µs | 0.406 µs |
| 2 | 0.062 µs | 0.371 µs | 1.288 µs |
| 3 | 0.068 µs | 0.443 µs | 1.174 µs |

The spread between runs of the same binary (three-fold) is larger than any effect this change could have on `update_mix`, which it does not call; iteration 117's band was 0.06 / 0.32 – 0.36 / 0.57 – 1.2 µs. Runs 2 and 3 sit in it (slightly above at 64 and 256: this machine is shared); run 1 is a quiet moment. No before/after claim is made from them beyond *no change that this instrument can see*.
