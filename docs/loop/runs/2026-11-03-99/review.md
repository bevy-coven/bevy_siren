# Iteration 99 — `#46`, the human listening check

## What was built

- `docs/guide/listening.md` — the procedure: the commands, the **order** to ask the five questions in, what counts as a
  failure, how to record the outcome.
- `examples/audition.rs` — puts the anchor and its neighbours in **rank order** as `01-first.wav … 0k-last.wav`, with a
  `README.txt` naming each file's hash and score.
- The first run, below.

## Why the files are numbered by rank and not named by asset

The question is *"does the ranking put the right thing first"*. A listener given `a.wav`, `b.wav`, `c.wav` in
alphabetical order cannot answer it. The numbers **are** the ranking.

## Why the script writes files instead of playing them

This crate's playback path wants a game's `App` and an audio device. A listening check whose script depends on the
machine having speakers is one that will not run over SSH or on a build host — and the check is supposed to be run by the
person who can hear, on the library they know, not by whatever machine happens to have an output device.

Copying rather than re-encoding is deliberate too: a listening check must hear the asset as it is.

## The first run — executed 2026-11-03

```
cargo run --release --features semantic --example audition -- /tmp/audiolib 3
wrote 3 files and README.txt to docs/loop/runs/2026-11-03-99/evidence/audition
```

| | |
|---|---|
| files written | `01-first.wav`, `02-1.wav`, `03-last.wav` |
| README | names each file's asset, hash, the anchor, the model stamp and the weights |
| the procedure | **exercised end to end** — the files exist, the names are rank order, the README is readable |

## The listening step: unperformed, and recorded as such

**Nobody listened.** The machine that built this cannot hear, and inventing an outcome would be the exact failure this
issue exists to prevent — a perceptual claim with no listener behind it.

So:

| criterion | state |
|---|---|
| the procedure is in `docs/guide/` and names the exact commands | **done** |
| the first run's outcome is recorded, including anything that sounded wrong | **the mechanical run is recorded; the listening outcome is not**, because there was no listener |
| a result that fails the check has a named follow-up issue | **not applicable yet** — nothing was heard, so nothing was filed |

## What this means for `#46`

The script, the procedure and the recording shape are in place, so the check is now *one command and five questions*
rather than a paragraph of intent. What remains needs a person with the library and ears, and it cannot be done from a
build host.

**The finding worth stating**: this is the only acceptance criterion in the whole backlog that this repository cannot
close by itself, and that is a property of the crate rather than of the loop — the audible half of "does it sound right"
has no substitute for hearing it.
