# Iteration 103 — perf

Machine: this Mac, **shared and loaded**. The variance below is the machine's, and this iteration claims no CPU change.

## Per-frame audio cost

No system reads the new code — `accepts` runs once per search, not per frame — so this is a control.

| voices | this run | iteration 102 | spread seen |
|---|---|---|---|
| 64 | 0.395 | 0.325 / 0.323 / 0.107 | 0.107–0.395 |
| 256 | 1.339 | 0.401 / 0.471 / 1.263 | 0.401–1.339 |

**No claim is made from these.** Iteration 102 recorded a **3.1× spread across three runs of unchanged code** at 256
voices, and this run's 1.339 is inside that same band. A single measurement on this host cannot resolve a 10 % change,
which is why the honest statement is *"unchanged, within a spread wider than any change this iteration makes"*.

## What the change costs

`Index::accepts` is a stamp comparison and an `Option::is_some_and` — a few pointer loads and an equality check on three
small fields, once per search call. The previous code compared stamps inline, so the work is **identical**; only the
place it happens moved.

There is nothing to benchmark: a predicate over two existing fields would produce a number smaller than the noise of
calling it.
