# plan.md — iteration 49: a bus's worst-case peak is readable

## The pick, and why it beats the runner-up

**Pick:** the crate could not answer *"can my table clip the device?"* `max_voices` is a sample cost, not headroom (S2),
`model::sum_db` already knows the arithmetic (S3), and `SirenMix` already holds both inputs — but nothing combined them,
so `examples/render` printed `(the budget's job, not the gain's)` **for 307 clipped samples**, which is false.

Beats the runner-up (case 31's blocked report sentence): that waits on a public enum; this is additive, and it is the
first thing found in several iterations that is about **sound output** rather than a report's wording.

## The blocked item

**Case 31** — still blocked on `needs-decision/bad-gain-problem.md`. Nothing built from it.

## The design

1. **`SirenMix::worst_peak_db(bus) -> f32`** — `n` live voices at full scale, plus the bus's gain, via `model::sum_db`.
   An **upper bound**; the doc says the crate never reads a sample's amplitude. `SILENT_DB` when no voice is live.
2. **The harness's false parenthetical is corrected**: *"past full scale — the budget is a sample cost, not headroom:
   `SirenMix::worst_peak_db` is how a game asks"*.
3. **The panel shows it** in the `Music` block's tail: `peak Effects +6.0 dBFS / 2  Music +4.0 dBFS / 2`. **Not** in
   `Problems`, which describes the table — a peak depends on this frame's voices. (The first draft put it in
   `Problems`; the capture showed the block saying `none` **and** a bus problem at once, which is a block answering two
   questions.)

## Fields

**No new field.** Both inputs existed; this is the arithmetic between them.

## Tests

- **`tests/mix.rs::worst_peak_is_the_linear_sum_of_the_live_voices`** — one voice at unity is 0 dBFS; two is **+6.0206**
   (the number R6 names); eight is **+18.0618**; a −6 dB bus with two voices is **+0.0206**; a bus with no voices is
  `SILENT_DB`; −20 dB of bus takes two voices below the rail.
- **A doctest** on the method, so the example in the docs cannot drift.

## The carve

- **Not** a limiter or an automatic gain: the crate's rule is that a table is the author's, and a limiter is a mixing
  decision with its own parameters. This reports.
- **Not** a `problems()` sentence: the table is not wrong. `Problems: none` stays true for the shipped table, and the
  peak is shown where the frame's state lives.
- **Not** an exact measurement: the crate has no sample amplitudes, and a number that pretended to be exact would be
  the very thing this iteration fixed.
