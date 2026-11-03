# research.md — iteration 46: a duck with neither a hold nor a fade ducks for no time, and nothing said so

## The item

`Duck::new(source, target)` leaves **`hold_s: 0.0` and `fade_s: 0.0`**. `active_duck_db` returns `by_db` while
`elapsed < hold` and, when `fade <= 0.0`, returns **`0.0`** for every frame past it. So a rule built that way — the
shortest thing a project writes — **fires, is recorded in `SirenLog`, and is inaudible on the very frame it fires.**

The crate's own `default_ducks()` chains `.hold_s(0.25).fade_s(0.6)` and `.fade_s(0.4)` on both of its rules, which is
why the pair every project starts from works and the hazard never showed. **`problems()` — nine sentences — said
nothing about it.**

## Sources

**S1. `Duck::new`** — *"`hold_s: 0.0, fade_s: 0.0`"*, and `Duck::hold_s`/`fade_s` are `serde(default)`, so a rule read
from RON that names neither is the same shape.

**S2. `model::active_duck_db`** — *"if `since < hold` return `by_db` … if `fade <= 0.0` return `0.0`"*. The mechanism,
and the reason a zero-length rule is exactly zero rather than small.

**S2b. `default_ducks`** — `Duck::new("Dialogue", "Music").by_db(-8.0).hold_s(0.25).fade_s(0.6)` and the `Effects`
rule with `.fade_s(0.4)`. **The crate's author chained both** — which is the evidence that the bare constructor is not
enough, and that nothing told a reader so.

**S3. `docs/guide/buses.md`** — *"a voice starting on `source` lowers `target` by `by_db` for `hold_s`, then lets it
recover over `fade_s`"*. A rule with neither runs for **no** time; the guide's own sentence already implies it.

**S4. `Duck::hold_s`'s doc** — *"How long the full reduction holds before recovery starts"*, `serde(default)` with
**no** default function — so `0.0`, unlike `by_db`, which has `Duck::default_by_db()` returning `-6.0`. One field
defaults to something an author would write; the other two default to inertness.

## The technique

A tenth sentence in `SirenSettings::problems`:

```
the duck on `Music` has no hold and no fade, so it lowers the bus for no time at all; set hold_s or fade_s
```

Guarded on `by_db < 0.0` so a rule that **raises** is not reported twice — it already has its own sentence, and the
"no hold" one would be noise beside it.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `Duck::new`'s hold | `0.0` | S1 |
| `Duck::new`'s fade | `0.0` | S1 |
| `Duck::new`'s `by_db` | `-6.0` (a real default) | S4 |
| the depth past the hold with `fade <= 0` | exactly `0.0` | S2 |
| the crate's own pair | hold `0.25`, fade `0.6` / `0.4` | S2b |
| the new sentence's guard | `hold == 0 && fade == 0 && by_db < 0` | **design default** — a raising rule is already named |

## What good sounds like

A `Problems` block containing `the duck on \`Music\` has no hold and no fade, so it lowers the bus for no time at all;
set hold_s or fade_s` for a table whose rule is inert, and **nothing** for the crate's `default_ducks()`, which sets
both. Measured: `tests/mix.rs::a_duck_with_no_hold_and_no_fade_ducks_for_no_time` gets `0.0` from the bare rule and
`-8.0` from the same rule with `.hold_s(0.25).fade_s(0.6)`.
