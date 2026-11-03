# Iteration 105 — perf

Machine: this Mac, shared and loaded. Nothing here is on the per-frame path.

## Per-frame audio cost (control)

| voices | this run | band seen across unchanged code |
|---|---|---|
| 8 | 0.070 | 0.041–0.067 |
| 64 | 0.334 | 0.107–0.395 |
| 256 | 1.284 | 0.401–1.339 |

Inside the band iteration 102 recorded, so no claim is made. The 8-voice figure is marginally above its previous range and
the change is a `matches!` in a function called once per table load, so it is the machine.

## What the change costs

`problems()` gains **one `matches!` per cue**, evaluated when a table is loaded or a project asks for its report — not per
frame and not per play. A `matches!` on a `Copy` enum is a discriminant test, so the cost is a comparison per cue in a
function that already walks the cue list several times.

There is nothing to benchmark, and inventing one would produce a number smaller than the timer's own overhead.

## The bug this costs nothing to avoid

A machine-gun footstep is a **design failure a listener hears immediately** and a developer debugs for an afternoon,
because every play succeeds. One comparison per table load is the cheapest possible place to catch it.
