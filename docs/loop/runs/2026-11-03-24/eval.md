# eval.md — baseline and the `cues.yaml` table, iteration 24

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 105 + 5 doctests green |
| `cargo test --features serde` | 112 + 5 green |
| `cargo test --features mixer` | 113 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 14 GB free (98 % — watched, since a release build wants ~2 GB). No open issues. Iteration 23's run
has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. **No case reads `SirenLog` after a
burst** — which is the gap this iteration closes.

## The lead, and what it became

Iteration 23's lead was the **`SirenLog` cap under a burst**. Probed, the *arithmetic* came back **clean**: capped at
256, oldest dropped, `recent(0)` and `recent(10_000)` both safe, no growth. Four other probes were clean too —
`line()` is never called in `src/` (host only); `next_boundary` at tempo `0`, negative, `NaN`, `1`, `1000`, and a
non-whole bar all match its doc; every no-ears spatial shape refuses or plays as documented; `SpatialMute` carries its
position.

The probe that paid asked a different question — **what a *refusal*'s lifetime is**:

| | before |
|---|---|
| one `Refused`, then plays | evicted after exactly **256 plays** |
| a 32-voice scene at 20 plays/s | 640 entries/s, so **0.40 s** |
| a frame of 300 plays in the plugin | the refusal is **gone** |

That is the silent-failure class the log exists to prevent: the line explaining why nothing played, discarded by lines
saying something did.

## Ranked

1. **A `Refused` entry is evicted by 256 routine plays** — the log's own purpose, defeated by a busy scene. **The pick.**
2. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
3. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. `Refusal::NotALoop` (`needs-decision/not-a-loop-refusal.md`).

## After the change

| case | after | green? |
|---|---|---|
| one refusal, then `CAPACITY` plays | the refusal is **still there**; len 256 | green (evicted before) |
| 2000 plays | still there — it leaves at 256 *routine* entries | green |
| 1000 mixed pushes / buffer size | len ≤ 256 throughout | green (guard) |
| 300 refusals | len 256, oldest `c44` | green (guard — the degenerate case) |
| order after a middle-class eviction | oldest-first, `c1` dropped | green |
| every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
