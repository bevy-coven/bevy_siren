# review.md — iteration 27, three passes

## Pass 1 — function

**The claim.** A duck is a rule about a **voice** — `Duck`'s own doc and `docs/guide/buses.md` both say *"a voice starting
on `source` lowers `target` by `by_db` for `hold_s`, then lets it recover"*. `DuckClock` ran the full hold whatever
happened to the source, so a line stopped on frame 0 kept `Music` down for another 15 frames.

| case | source | before | after |
|---|---|---|---|
| the line is **stopped** on frame 0 | S1 (`Duck`'s doc), S2 (Gregory §14.6.3 via R3) | full hold, `+0.0` at **frame 60** | fade from the stop, `+0.0` at **frame 40** |
| a **loop** keeps playing | S1, S3 | full hold, `+0.0` at 60 | **unchanged** — 60 |
| a **second line** at frame 20 | S3 (`fire_all` re-arms) | extended, not zero by 70 | **unchanged** |
| a **one-shot that ends by itself** | S4 (`voices_on` excludes a fading voice) | `+0.0` at 60 | **unchanged** — 60 |

**Against the sources.** S1 is the whole argument: the hold is bounded by the voice, and the crate's own doc says so. S4
is why the one-shot row must **not** move: its voice is still live until `retire_voices` despawns it, so the hold is
still justified. Only a voice the player can no longer hear — a stopped one — releases the hold early. The diff is
**scoped to one row of four**, which is what a correct fix looks like.

**The severity:** the skill names *"a duck that does not recover"* as rank-1. This was that failure arriving late: 0.85 s
of duck for a sound that had already stopped.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/duck-hold{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**A defect in my own first fix, caught by the probe.** The first version clamped the elapsed time in `duck_db`:
`since = max(elapsed, hold)`. That pins the fade at exactly `hold_s` **every frame**, so it never advances — the duck
held **forever**, worse than the bug. The debug print showed `since=0.250 db=-8.00` on every frame:

```
[dbg] rule Dialogue->Music empty=true elapsed=0.032 since=0.250 db=-8.00
[dbg] rule Dialogue->Music empty=true elapsed=0.048 since=0.250 db=-8.00
```

The fix is a **one-time rewind** of `started` in `source_went_quiet`, so the fade is genuinely underway and keeps
advancing. The probe caught it because the numbers did not move between frames.

**A second defect, caught by clippy:** `duck_db_with` survived the rewrite as a method nothing called with a real map —
a vestige. Deleted; `duck_db` is the single read.

**Audio unchanged for every table without a stopped duck voice.** 21/21 WAVs byte-identical; `mix.wav` −4.3 LUFS /
+0.2 dBFS.

**Verdict: GREAT** after both corrections (pass 1 re-ran).

## Pass 3 — fresh eyes

**A different shape.** Four cases in one probe, differing only in how the source voice ends: stopped, looping, restarted,
self-ending. That is the shape a duck's contract needs — the interesting question is not the envelope but **which ending
releases the hold**, and the table answers it row by row.

**Can a developer understand it without the code?** `source_went_quiet`'s doc says what it does and why the rewind is
once-only, and names the measured failure it prevents. `duck_db`'s comment says the rewind happened before the read. A
reader who sees a duck release early has one place to look.

**What a fresh reader would still ask:** *why not a `Recovered` log entry?* Because it is a new variant of a public enum,
the same break as the four `CueProblem`s; `needs-decision/duck-recovers-entry.md` records it, recommends **against** it
(the release is `SirenMix::ducked == 0.0`, and one line per release would double duck traffic in a 256-entry log), and
says what a text-only panel cannot see.

**Verdict: GREAT.**
