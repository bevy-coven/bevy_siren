# Iteration 88 — review

## Pass 1 — Function: does the bench answer #36?

**Critique.** Three of four criteria are met from a measurement on this machine; the fourth needs the GPU host.

| # | Criterion | State |
|---|---|---|
| clips per second **and** peak VRAM, with the machine's state | **clips/s measured**; VRAM **not**, and the reason is below |
| the 5 000-clip projection in minutes | **3 s**, from a rate measured at three corpus sizes |
| the bound (decode or inference) identified | **partly, and honestly** — see below |
| the target written down with the number justifying it | the rate is the number; a *target* needs the GPU half |

**The bound is the subtle one, and the bench refuses to overclaim.** Its own `embed*` stage is **262 144
multiply-adds, not the model**, and it says so on the line below the table. ADR 0002 measured the real audio tower at
**34.9 ms per clip** on an M4 CPU; against a decode of **0.37 ms/clip** that makes inference dominate by ~100×. So `#36`'s
third criterion is answered by **comparing two measurements** — one from this bench, one from ADR 0002 — rather than by
the bench's synthetic stage, and the file states which is which.

**The decode figures are stable** — 2 411 / 2 636 / 2 724 clips/s at 200 / 500 / 1 000, within 10 % — which is what makes
extrapolating to 5 000 defensible. A single-size measurement would not be.

**Verdict: GREAT** for the decode half, with the GPU half explicitly open.

## Pass 2 — Zoom: the fixture that lied twice

**Critique.** Two independent mistakes that together produced a confident, wrong report.

**The fixture.** `tone()` set `hz = 220.0 + (seed % 24) * 40.0`, so seeds 0, 24 and 48 wrote **identical bytes**.
`shasum -a 256 c0000.wav c0024.wav c0048.wav` returns one hash. The scan then correctly reported `assets 24, skipped 0,
refused 0` — 24 contents, each at several paths. **The scan was right and the corpus was wrong**, which is why the
diagnosis had to start with `library::scan` rather than with the scan's code.

**The report.** The bench printed `indexed {decoded.indexed()} / {n}` where `n` was **also** `decoded.indexed()` — a
ratio whose denominator comes from its numerator, which can never fail. On a 200-file corpus it printed `indexed 24 / 24`
and three projections that were all nonsense.

**Fixed both**: every seed now produces unique bytes (frequency advances with the seed, plus a `1/√2` phase offset so
near-equal frequencies still differ), and the denominator is the **corpus size**, so a mismatch is visible as
`24 / 200`.

**The general lesson**: a fixture that produces fewer distinct inputs than it claims is invisible unless the report
compares against a count the fixture did not produce. That is the same shape as iteration 87's constant-vector bug —
*a fixture chosen for convenience can make a true claim fail, or a false one pass.*

**Verdict: GREAT.**

## Pass 3 — Fresh eyes: can a developer use it?

**Critique.** One command, and it makes its own corpus:

```sh
cargo run --release --features semantic --example indexbench -- /tmp/library 1000
```

Point it at a real library instead and it uses it as given. The output names what each column means, says the embed
column is synthetic **on the line under the table**, and prints the projection for 500 and 5 000 clips rather than
leaving the arithmetic to a reader.

**The GPU half, stated plainly rather than buried**: `big` is reachable (RTX 3090, 24 576 MiB, 24 cores, confirmed this
iteration) and holds a checkout at **`8e14231`** — older than every semantic change since iteration 62, with **no ONNX
graph present**. A GPU number needs the current commit, the 116.5 MB graph, and `ort` built against CUDA. Reporting a
number from a stale checkout would be exactly the "aspiration" `#36` forbids, so **`#36` stays open** and this iteration
ships the half that does not need the accelerator.

**Verdict: GREAT.**
