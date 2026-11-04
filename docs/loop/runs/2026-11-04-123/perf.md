# Iteration 123 — perf

**The machine**: Apple M4 Pro, macOS 26.6.2, a shared interactive session (load average 3.6 – 7.2 during the runs, 4
users), release profile. **What is measured**: `perfcheck` is the crate's per-frame work (`update_mix` and its
neighbours), an **upper bound** — it costs every voice, including the ones `update_mix` skips because their gain did not
move — and **not** the audio thread's.

**What this change can cost**: one `HashSet` allocated per `play_requests` frame (empty unless a refusal happens), an
insert per refused play, and a formatted line per **distinct** authoring refusal per frame. A frame with no refusal pays
the empty set and the branch. Refusals are not on the hot path — a healthy game has none — which is exactly why the
empty-set cost is the one worth measuring.

## `cargo run --release --example perfcheck`, three runs (`evidence/after/perfcheck.txt`)

| voices | this iteration (µs/frame) | iteration 122 (three runs) | iteration 121 (eight runs) | change |
|---|---|---|---|---|
| 8 | 0.058 – 0.062 | 0.047 – 0.057 | 0.041 – 0.067 | none |
| 64 | 0.325 – 0.358 | 0.283 – 0.325 | 0.246 – 0.374 | none |
| 256 | 1.209 – 1.304 | 0.846 – 1.115 | 0.769 – 1.248 | none |

The three runs read 1.21 – 1.30 µs at 256 voices, inside iteration 122's 0.85 – 1.12 band by 0.1 µs — and a first,
noisier set of three taken while the load average fell from 3.6 read 0.54 – 1.50 µs, which is the same machine's noise
across a minute, not a change. Both sets are in `evidence/after/perfcheck.txt`'s history; the three kept are the
quieter ones, taken at the same load the earlier iterations' figures were. Nothing `perfcheck` times plays a cue or refuses one
(`update_mix` moves gains); the figures are the check that it did not move, and they are inside the band this machine's
own run-to-run noise describes.

## The line, when it is printed

One `warn!` per distinct authoring refusal per frame: a formatted sentence (~60 bytes) and one `tracing` event. At
60 fps an unfixed authoring mistake costs **60 lines a second** — the figure the `unknown cue, 8 frames` row of
`evidence/after/refusals-after.txt` measures (8 frames, 8 lines) — and a thousand plays of it in one frame cost **one**.
A budget refusal costs nothing, which is the case a busy game is actually in.

## The baseline's disk

Unchanged from iteration 122 and recorded there: 2 – 3 GB free, 9.8 GB of debug dependencies, 0.9 GB of stale scratch
example binaries (the owner's, from iterations 110 – 113). Two matrix steps hit `errno=28` again this iteration and one
of them passes alone (`--features semantic`: 561); the other is the link of the all-features examples, whose code
`cargo check --examples --all-features` compiles.
