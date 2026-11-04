# Iteration 108 — review

## Pass 1 — function

**The defect.** `problems_names_each_mistake` is the test whose job is *every problem a table can have is named, in a
sentence*. `CueProblem` had seven variants; the test produced six. `InertAvoidRepeat` (iteration 105) was tested on its own
— `tests/play.rs` asserts it fires and that legal windows and `Shuffle` do not, `tests/model.rs` proves the behaviour — but
no fixture cue here produced it, so it was never run through the *"is a sentence, not a debug print"* loop, and the suite
stayed green.

| criterion | evidence |
|---|---|
| the fixture produces all seven kinds | `problems_names_each_mistake` passes; the `inert` cue and its row are new |
| a variant added to the enum | **`E0004 … &CueProblem::Tenth { .. } not covered`** at the macro's `match` |
| a variant added **and listed**, no fixture | **fails**: `no fixture cue produces ["Tenth"] — every variant needs one` |
| **the real historical state replayed** — `InertAvoidRepeat` listed, its cue and row removed | **fails**: `no fixture cue produces ["InertAvoidRepeat"]` — iteration 105's exact gap, on the real variant |
| `src/cue.rs` untouched by the experiments | content hash back to its original value after both reverts |
| nothing else moved | 199 / 208 / 209 / 109 + **436** / **460** / 6 doc green; clippy, fmt, docs clean |

**Critique before the verdict.** Three limits, and the third is the one worth stating:

1. **`kinds!` is still one hand-written list.** Forgetting to list a new variant is a compile error, so the list cannot
   drift *short*; a variant listed twice is an `unreachable pattern` warning, which this project's `-D warnings` clippy turns
   into an error. So it cannot drift either way — but it is a list a maintainer edits, which is why it sits beside the
   fixture and is named in the failure message.
2. **It covers what `problems()` returns for this fixture.** A future variant produced only by `problems_in(settings)` (which
   needs a settings table) would need its own fixture; `problems()` delegates to `problems_with(&DEFAULT_BUSES)`, so every
   current variant is reachable.
3. **An earlier design of mine did not do this**, and I believed it did (see pass 3).

**Verdict: GREAT.**

## Pass 2 — zoom

Read `spectrograms/footstep-stone.png`, the cue case 2 clip (a 0.33 s stone step, `avoid: 4` over two sounds).

- **Attack:** a broadband burst from 0 to about 0.027 s, reaching past 10 kHz. It ends on a sharp vertical edge, and the
  edge is the one place a click could hide — but there the broadband content sits near **−90 dBFS** on the colour scale,
  roughly 40 dB under the partials beside it, so the step is below audibility. It is the transient's own envelope.
- **Body:** three sustained low partials at roughly 0.3, 0.8 and 1.5 kHz that decay slowly to the end of the clip; the upper
  partials fade progressively (the 2–3 kHz ones last to about 0.25 s, the highest are gone by about 0.1 s). No vertical
  line after the attack, no flat shelf, no hole.
- **Tail:** the clip ends with the low partials near −95 dBFS, so any discontinuity at the end is far below audibility.
- **Peak** 0.429 / 0.431 (about −7.3 dBFS) — no clipping edge.
- **All 21 rendered wavs byte-identical** to iteration 107's (`cmp`, no differences); master **−21.5 LUFS, −7.5 dBFS true
  peak** (`ebur128=peak=true`, re-read this pass).

This is the expected result — `src/` did not change — and it is the point of the zoom: a test-only iteration must leave every
sample byte-identical.

**Verdict: GREAT.**

## Pass 3 — fresh eyes

**My first design was wrong, and I only found out by reading `git diff`.** I wrote *"no guard exists"* and added a second
exhaustive `match` in the test; I added a tenth variant, the test stopped compiling, and I counted that as proof. It was not.
`HEAD`'s `named` closure already listed every variant with no wildcard, so `HEAD` stops compiling in exactly the same way —
and it **had already done so** in iteration 105, which is the `E0004` quoted in `needs-decision/diagnostic-enums-are-extensible.md`.
That firing is *how the gap happened*: the compiler demanded an arm, I wrote the arm, and nothing demanded a fixture.

So the experiment I ran proved only that Rust has exhaustive matches. The experiment that **separates the new guard from
the old one** is *"listed, with no fixture"*, which `HEAD` passes and the new test fails — and the strongest version of it is
iteration 105's own state replayed on the real variant, above.

Two more findings from the same reading, both corrected in the record:

- the `named` closure's comment said it did not enumerate the variants — it enumerated all seven;
- its assertion was vacuous, implied by `problems.contains(&problem)` one line above (that compares the name too).

I deleted the closure rather than keep a third copy of the list that checked nothing. And iteration 105's decision file
claimed the test had been *"fixed to match the documented property"*; it had been given an arm. That paragraph now says so,
and records the one cost the file had missed: the `all` test crate **does** honour `#[non_exhaustive]`, so the `kinds!` match
would need a `_ =>` arm (a one-line change that keeps the coverage half).

My first draft of this iteration's own notes also overstated the hole as *"a variant nothing ever tests"*. `tests/play.rs`
and `tests/model.rs` test it. The gap is the table-level property, and the notes now say that.

Read as a maintainer: add a variant, get a compile error naming it; list it, get a test failure naming it again; add the
fixture cue, green. Neither step can be satisfied by writing an arm. Read as a reviewer: the diff is one macro, one fixture
cue, one row, and a deleted closure.

**One criticism, recorded:** this is the third of three iterations spent on lists that can drift. Each was real and each was
found by counting — and none changed a sound. With every remaining issue blocked outside this repository, that is the honest
shape of the work left, not a reason to call it more.

**Verdict: GREAT.**
