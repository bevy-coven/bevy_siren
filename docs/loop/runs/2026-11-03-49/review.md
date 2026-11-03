# review.md — iteration 49, three passes

## Pass 1 — Function: does the number say what the source says?

**The source.** R6 in `docs/research/siren.md`: *"several sounds **sum in linear** (two 0 dB sounds are one at +6.02 dB)"*.

**Measured against it** (`tests/mix.rs::worst_peak_is_the_linear_sum_of_the_live_voices`):

| input | `worst_peak_db` | R6's number |
|---|---|---|
| one voice, bus 0 dB | **0.000** | full scale |
| two voices, bus 0 dB | **+6.0206** | *"+6.02 dB"* — exact |
| eight voices, bus 0 dB | **+18.0618** | 8× |
| two voices, bus −6 dB | **+0.0206** | measured, in the doctest too |
| two voices, bus −20 dB | below 0 dBFS | the number's purpose |
| no voices | `SILENT_DB` | "nothing to hear" |

**And the harness's claim is corrected.** `voices-8.wav` has **307 samples at the rail**, measured from the WAV itself
(`peak 1.000 of full scale`), against a line that called it *"the budget's job"*. `max_voices` is *"fewer than a mixer's
worth of sample cost"* — S2 — and always was.

**Verdict: GREAT.** The arithmetic matches the crate's own stated rule to four decimals, and a false sentence is gone.

## Pass 2 — Zoom: read the crop closely

Read: `evidence/panel/headroom-40.png` at two revisions, and `evidence/after/render.txt`.

**A defect found and fixed by looking.** The first version put the peak in the **`Problems`** block, which then said
`none` *and* `bus \`Effects\` can reach +6.0 dBFS: 2 voices` — a block answering two questions, and one of them about
the current frame rather than the table. Moved to the `Music` block's tail, where the frame's mix already lives. The
second capture shows `Problems: none` and `peak Effects +6.0 dBFS / 2  Music +4.0 dBFS / 2` — one idea per block.

**A second defect, in my own edit.** The move left the old `Problems` line in place twice — `cargo fmt` had reflowed the
block and both `replace`s silently missed. Caught by **re-reading the capture** after the change (the sentence was still
there) rather than by assuming the edit landed.

**No wrapping.** At 1280 the `Music` line is `calm −4.0 dB  combat −80.0 dB  peak Effects +6.0 dBFS / 2  Music +4.0
dBFS / 2` — one row, no second line, and the log keeps six lines.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: the number on a table that is fine

The shipped table is **clean** — `Problems: none` — and it still clips the moment two voices share a bus. That is the
fresh-eyes lesson: a report that says `none` is not a report that the mix is safe, and those are now said separately
where a reader can see both at once.

**Can a developer understand it without the code?** Yes: `peak Effects +6.0 dBFS / 2` names the bus, the bound and the
voice count that reaches it. A developer who wants −1 dBFS of headroom lowers the bus 7 dB, and the number follows.

**Verdict: GREAT.**
