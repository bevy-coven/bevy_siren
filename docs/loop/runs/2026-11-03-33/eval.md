# eval.md — baseline and the `cues.yaml` table, iteration 33

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 113 + 5 doctests green |
| `cargo test --features serde` | 120 + 5 green |
| `cargo test --features mixer` | 122 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 13 GB free (98 %). Iteration 32's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. Cases 15 and 23 drive a score at a
**fixed** tempo, and case 15's `green_when` is *"the target moves on a multiple of 1.5 s"* — 1.5 s being a bar at 160 bpm.
**No case edits the tempo.**

## The lead, and what it became

Iteration 32's lead was a **Music block** in the panel, with its height budget measured at 12 px free against ~134 needed.
Probing the layout to choose rows for the trade, I measured the panel's own structure — and then probed the **music**
behaviour the block would have shown, which found something larger:

| probe | result |
|---|---|
| `NextBar` queued at 160 bpm, tempo unchanged | fires at 1.44 s — a bar at 160 is 1.5 ✔ |
| `NextBar` queued at 160, score changed to **80** | fires at **1.44 s**; a bar at 80 is **3.0** — **mid-bar** |
| `NextBar` queued at 160, score changed to **240** | fires at **1.44 s**; a bar at 240 is **1.0** — **a bar late** |

`Pending` stored only `at_s`, computed once from the tempo at request time. `MusicScore` is a resource the Cauldron edits,
so a tempo change while a change waits is the ordinary path — and iteration 14 fixed the same class for **layers** while the
**pending queue** kept its old tempo.

## Ranked

1. **A queued `NextBar` change ignores a tempo edit** — the crossfade lands off the beat, in both directions. **The pick.**
2. **A Music block** in the panel — rank 4, and it needs a layout trade measured at 12 px free against ~134. Deferred again,
   deliberately, with the budget recorded.
3. A `Stopped` line for a parent's despawn (blocked on the public-enum decision, seven items deep).
4. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
5. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).

## After the change

| case | after | green? |
|---|---|---|
| 160 → 80 bpm before the bar | fires at **2.944 s** — on the bar | green (1.44 s, mid-bar, before) |
| 160 → 240 bpm before the bar | fires at **0.944 s** — on the bar | green (1.44 s, a bar late, before) |
| tempo unchanged (guards) | 1.44 / 2.944 — identical | green |
| a `When::Now` change | applies on its own frame | green |
| every test, both feature sets | 116 / 123 / 125 + 5 | green (+3 on each) |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
