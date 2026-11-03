# Iteration 95 — research: one transform per frame

## The item and why it ranks where it does

The log's own "top 5 remaining items" has, at position 5, *"Share the spectral FFT with `mel::log_mel` — iteration 89's
55 % decode regression, now with two more passes."* Iteration 93 measured the cost: **decode fell from ~1 150 to 419.8 /
568.8 clips/s** when tempo and key were added. Nothing in the crate is slower relative to what it should be.

## Sources

| Source | Number it reports |
|---|---|
| Press, Teukolsky, Vetterling & Flannery, *Numerical Recipes*, 3rd ed., §12.2 (FFT) | a radix-2 real transform is `O(N log N)`; recomputing an *identical* transform is the whole cost again, with no accuracy benefit |
| Harris, *Optimizing Software in C++*, §7 (caching and locality) | allocating inside an inner loop costs a `malloc` per iteration and defeats reuse; hoisting it out is the standard fix |
| `~/.cargo/registry/src/index.crates.io-*/rustfft-*/src/lib.rs` | the real-to-complex plan can be reused across frames — the crate's own `real_to_complex` API |
| Local, measured this iteration (see below) | spectral 3.96 ms, tempo 4.23 ms per 4 s clip, both walking identical frames |

## The technique

`spectral` (iteration 89) and `tempo` (iteration 91) each built a Hann window of `WINDOW = 1024`, stepped it by the same
`HOP = 480`, took the same 1024-point FFT, and each summed `|z|²`. They were the same computation, performed twice. The
fix is not an optimisation of the transform: it is to compute it once and hand the result to both.

`key` (iteration 92) is **not** in this share for a different reason: its chroma needs an 8192-point window, which is a
different transform, not a duplicate one.

## A second defect found on the way

`tempo::frame_energy` allocated `re` and `im` — two `WINDOW`-sized `Vec<f64>` — **inside** the frame loop. A 4 s clip is
400 frames, so 800 allocations of 16 KB. Hoisting them out is free and is part of this change.

## Parameters

| Parameter | Value | Source |
|---|---|---|
| `WINDOW` | 1024 | iteration 89's mel module, unchanged |
| `HOP` | 480 | same |
| window | Hann | same |

No new tunable is introduced: this is a refactor of where work happens, not of what it computes. There is therefore no new
field on `CueTable`, `SirenSettings` or `MusicScore`, and no design default to justify.

## What good looks like

Identical output, less time. The proof is a test suite that was written against the old numbers and still passes
**unchanged** — 10 spectral tests and 13 tempo tests, including the 80-tempo corpus — plus a direct A/B measurement of the
two call shapes.
