# review.md — iteration 50, three passes

## Pass 1 — Function: is the master the trim the decision file says it is?

**The claim** (`needs-decision/above-unity-gain.md`): *"The master's own level (`SirenSettings::master_db`) is the tool
for it."*

**Measured** (`tests/mix.rs::the_master_is_inside_the_worst_peak`), one voice at unity on a 0 dB bus:

| `master_db` | `worst_peak_db` |
|---|---|
| `+0.0` | **`+0.0 dBFS`** |
| `−6.0` | **`−6.0 dBFS`** |
| `−20.0` | **`−20.0 dBFS`** |

1:1, within `1e-3`. The claim holds, and now has a test rather than a sentence.

**Applied.** `examples/mixer`'s `Effects` `0.0 → −7.0` (`2 × 10^(−7/20) = −0.98 dBFS`) and `Music` `−2.0 → −7.1`
(`−1.08`). The panel's peak clause **disappears** — `peak Effects … Music …` was there, and is not, because nothing
reaches 0 dBFS. A signal that vanishes when the problem is fixed is the correct signal.

**Verdict: GREAT.**

## Pass 2 — Zoom: read the numbers as numbers

**A first attempt that was wrong and caught by reading.** I first set `Music` to `−5.1` and read `peak Music +0.9 dBFS / 2`
off the capture — my arithmetic had said −1.0. Recomputed: `2 × 10^(−5.1/20) = +0.92`. `−7.1` is `−1.08`. The panel,
not the arithmetic, was right; the arithmetic was mine.

**A second, found the same way.** `Effects` showed `+0.0 dB` after I thought I had set it to `−7.0`: the line carries an
`if overbudget` conditional, so the `replace` missed. Caught by **reading the bus row in the capture**, not by trusting
the edit.

**`mix.wav`.** Before: `+0.2 dBFS` true peak, `−4.3 LUFS` — the file was `sum_db` of ten cues (**+18.58 dB**) applied to
one 0.5-amplitude tone, i.e. **4.2× full scale**. After, written at `master_db`: **`−6.1 dBFS`, `−18.6 LUFS`**, which is
below the −14 LUFS a game mix usually targets. Its report line now says what the number is and what the file is.

**A fudge I wrote and deleted.** My first fix was a `master_trim(summed, master)` helper clamping to `−3.0` — a magic
constant doing an unclear job, and precisely the kind of thing this loop's rules forbid. Replaced by writing the file at
`master_db`, which is the field the decision file names.

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: does it still hold on a table that is not trimmed?

`examples/render`'s own table is **not** trimmed — it uses `SirenSettings::default()` — and `voices-8.wav` is still
`+0.1 dBFS`, deliberately: eight synthesised sources summed to exercise exactly that. Its line says the budget is a
sample cost and names `worst_peak_db`. So the two harnesses now agree about what clipping is and disagree about whether
to avoid it, each for its own reason: the panel demonstrates a healthy mix, and the render harness demonstrates the
arithmetic.

**Can a developer understand it without the code?** Yes — the guide's headroom section names the field to turn, gives the
1:1 relationship, and says why the report stays silent. A table that clips wants a decibel of master.

**Verdict: GREAT.**
