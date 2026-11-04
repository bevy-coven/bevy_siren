# Iteration 115 — perf

Four costs, because `examples/perfcheck` measures only one of them. Release builds, this Mac (M4 Pro, under load from nothing else). Every number is in `evidence/after/`.

## 1. `perfcheck` — the mix, which this change does not touch

Three runs (`evidence/after/perfcheck.txt`), µs a frame:

| voices | run 1 | run 2 | run 3 | iteration 114's band |
|---|---|---|---|---|
| 8 | 0.057 | 0.039 | 0.057 | 0.018 – 0.074 |
| 64 | 0.333 | 0.237 | 0.266 | 0.106 – 0.395 |
| 256 | 1.220 | 0.887 | 0.411 | 0.396 – 1.339 |

Inside the band at every size. **What this says**: nothing else moved. It does not run `trim_ogg_loops` or a loop's start, so it is not the cost of this change; sections 2 – 4 are.

## 2. `trim_ogg_loops` — a scratch test in release, removed (`evidence/after/trim-system-bench.rs.txt`, `trim-system-cost.txt`)

| voices | the frame they are first seen (µs, per voice) | every voice marked (µs a frame) |
|---|---|---|
| 8 | 90.25 (11.281) | 0.089 |
| 64 | 42.75 (0.668) | 0.079 |
| 256 | 90.83 (0.355) | 0.066 |

The first frame includes the page walk of a 5.7 KB Ogg for every loop and one `LoopChecked` insert (an archetype move) per voice, **for a one-shot as well**; the 8-voice figure is dominated by the system's cold first call. Once every voice is marked the query matches nothing and the cost is the system's own overhead. A loop whose asset has not loaded is looked at every frame until it does, at an `Assets::get` each.

## 3. `ogg::inspect` — the page walk (`evidence/after/ogg-loop-report.txt`)

| file | bytes | µs a call |
|---|---|---|
| a typical Kenney effect (median of 254) | 13 905 | **0.03** |
| the slowest of the 254 | 188 542 | 0.53 |
| a 300 s stereo 48 kHz Ogg (`ffmpeg -c:a vorbis`) | 9 144 647 | 8.2 |

Prediction P6 was *< 100 µs for an effect, < 2 ms for 5 MB*: right, loose by three orders of magnitude. The walk reads a 27-byte header and a segment table per page and skips the payload by arithmetic.

## 4. The decoder swap: `lewton` for `symphonia`'s Vorbis (`evidence/after/ogg-dump.rs.txt`, run on both)

All 254 of Kenney's Ogg files, decoded with `rodio::Decoder::new(Cursor)` into a `Vec<f32>`:

| decoder | samples | seconds | million samples a second |
|---|---|---|---|
| `symphonia` | 19 960 896 | 0.48 | 41.4 |
| `lewton` | 19 791 015 | 0.86 | 22.9 |

**1.8× slower, 240× real time for stereo 48 kHz**, and 0.38 s for the whole 254-file library. Indexing the 303-clip Kenney library with the real CLAP model took **27.2 s (11.2 clips/s)**, which the audio tower dominates. `the_decode_path_meets_its_throughput_target` passes.

**Not measured**: the per-sample cost of `looped`'s `Box<dyn Source>` on the first pass of a loop; CPU on the audio thread with a real device; memory of a decoded loop (unchanged by this change: `Repeat` keeps it, as before).
