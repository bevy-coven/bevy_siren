# Iteration 107 — review

## Pass 1 — function

**The defect.** `MusicScore::problems()`'s doc said *"Nine checks, and this list is the whole of them"*. The nine were
correct — and correct **by hand**, checked only when someone counted. Measured: adding a tenth check to `problems` left the
suite green and the sentence still claiming completeness.

| criterion | evidence |
|---|---|
| the doc names one list | `MUSIC_CHECKS`, first line of the doc |
| the test walks it | `needles.len() + DEDICATED_FIXTURE.len() == MUSIC_CHECKS.len()` |
| a tenth check with its name fails | **proved**: `9 != 10`, naming the whole list; reverted |
| the dedicated-fixture entry is honest | asserted to still be in `MUSIC_CHECKS`, so a deleted check cannot leave a stale exemption |
| nothing else moved | 9 needles still pass; 436 semantic, 460 all-features green |

**Iteration 106's criticism was half wrong, and the record now says so.** It claimed `MusicScore::problems` was *"one
inspection away from the same drift"*. There **was** a test, with nine needles, whose own doc records that the list once
said four while the code performed nine. The gap was narrower: the counts agreed, but nothing kept them agreeing. Stating
that precisely matters — a criticism that overstates is as misleading as a doc that does.

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/music-calm.png`.

- A 1.55 s looping bed: broadband attack at 0 s, then **flat sustained partials to the end with no decay** — correct for a
  loop, which is the one clip shape where a missing decay is right.
- **No vertical seam** at the wrap point, and no clipping shelf. On a loop that is the defect that would show.
- **All 21 rendered wavs byte-identical** to iteration 106; master **−21.5 LUFS, −7.5 dBFS true peak**.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**A measurement of my own was wrong, and it is the most useful thing in this iteration.** My first attempt to prove the
guard added the tenth **check** to `problems` without adding a tenth **name** to `MUSIC_CHECKS` — and the test passed. I
wrote that the guard had failed. It had not: the constant *is* the list, and the assertion compares needles against **the
constant**, so ten checks and nine names is a state the constant does not describe. Adding both fails at `9 != 10`.

That distinction is the design rather than a loophole — **a check that is not in `MUSIC_CHECKS` is not a documented check**,
and the constant is the one place a maintainer declares one — but I only understood it by watching my own experiment give
the "wrong" answer and following it instead of declaring victory. The research file now records the mistake in its own
section rather than quietly correcting the sentence.

Read as a maintainer: add a tenth check, add its name, and the test fails printing the list. Read as a reviewer: the diff is
one constant, one rewritten doc paragraph, and a count that used to be prose.

**One criticism, recorded:** `CueTable::problems` is the third of three with this shape and still carries no constant. Two
iterations have now done this pattern one type at a time; the third is mechanical and remains the recorded follow-up.

**Verdict: GREAT.**
