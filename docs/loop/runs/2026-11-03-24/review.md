# review.md — iteration 24, three passes

## Pass 1 — function

**The claim.** `SirenLog` exists so a silent failure can be read afterwards — `CAPACITY`'s own doc says its 256 entries are
*"for a person asking what just happened"*. It evicted the **oldest entry whatever it was**, so one `Refused` was pushed
out by exactly `CAPACITY` plays.

| row | source | before | after |
|---|---|---|---|
| one refusal, then `CAPACITY` plays | S1 (R6: *"a play is never silent: `Played` or `Refused`, both with the reason"*), S4 (`Refused` is exceptional) | evicted after **256 plays** — **0.40 s** at a 32-voice scene | **survives 2000 plays**; leaves when 256 *routine* entries accumulate |
| 600 refusals | S2 (256 is the budget) | len 256, oldest `c344` | len 256, oldest `c344` — **unchanged** (the guard) |
| 1000 mixed pushes | S2 | len ≤ 256 | len ≤ 256 — unchanged |

**Against the sources.** S4 is the whole argument: `Played`, `Stopped` and `Ducked` happen at frame rate in a busy scene;
a healthy game produces **no** `Refused` at all. A buffer that treats them alike optimises for the wrong stream. S3's
virtual-voice analogy is exact: a refusal is a logical fact that must outlive the physical channel.

**The severity:** this is the silent-failure class the crate exists to prevent — the line explaining *why nothing
played*, thrown away by lines saying something did, exactly when a busy scene made it matter.

**Verdict: GREAT.**

## Pass 2 — zoom

**Read:** `evidence/refusal-lifetime{,-before}.txt`, the spectrograms, the loudness table, three perf runs.

**Audio unchanged, and it must be.** The log is not the mix: 21/21 WAVs byte-identical; `mix.wav` −4.3 LUFS / +0.2 dBFS.

**The measured table**, `evidence/refusal-lifetime{,-before}.txt`:

```
before                                    after
1 refusal + N plays: gone at N = 256       survives 2000 plays
  -> 0.40 s at 640 entries/s                -> leaves at 256 ROUTINE entries, >= 0.40 s and longer quietly
600 refusals: len 256, oldest c344         len 256, oldest c344   (unchanged)
```

**A correction in my own evidence, caught here:** the first version of the probe printed *"3.12 s"* for the after case by
falling back to its 2000-play limit — a made-up number for a behaviour that has no fixed play count at all. It now says
what the rule is (*256 routine entries*) and the floor that follows from it.

**The cost, measured rather than assumed:** 200 000 pushes of routine entries took 11.1 ms (55 ns each — the scan finds
index 0 immediately, because the oldest entry is routine); 1000 pushes into an all-refusal log took 272 µs (272 ns each,
the pathological case where a game is already failing loudly). No optimisation needed, and the numbers are in `perf.md`.

**Verdict: GREAT** after that evidence correction (pass 1 re-ran).

## Pass 3 — fresh eyes

**A different shape.** Four tests: the burst (the fix), a mixed-push capacity guard, an all-refusal degenerate case, and
an **order** test after a middle-class eviction — which is the one a panel depends on, because `iter()` feeds a column.
The order test asserts the dropped entry **by identity** (`c1` at 1.0 s is gone, `c2` at 2.0 s is first), not merely that
the list is sorted.

**Can a developer understand it without the code?** `push`'s doc states the rule, names the exceptional class, and gives
the measured 256-plays figure so the change has a reason a person can check. `CAPACITY`'s doc is unchanged and still
true.

**What a fresh reader would still ask:** *why not keep refusals forever?* Because the buffer is one fixed 256 and a
growing one is a new public type; if every entry is a refusal the oldest still goes, and 256 refusals is a game already
failing loudly. Written in the doc and tested.

**Verdict: GREAT.**
