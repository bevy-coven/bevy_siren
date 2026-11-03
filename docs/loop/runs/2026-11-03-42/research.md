# research.md — iteration 42: which cap took the voice, and how many of this cue may sound

## The item, in two parts

**First, `max_instances` was invisible.** The cue list showed name, bus, mode, gain and priority. `priority` decides
who **survives** a full budget; `max_instances` decides whether the **next play may start at all**. A reader looking at
`footstep.grass … priority 10` had no way to know two footsteps may overlap and a third takes the first back — while a
`ui.click` at `x1` may not.

**Second, and found by making the first work:** a steal said `stolen by footstep.grass` and **not which cap** took the
voice. Three numbers produce that sentence — the cue's `max_instances`, its bus's `max_voices`, the table's
`max_voices` — and each wants a different edit from an author. A step that took three voices logged **two identical
lines**, and neither said which number to raise.

## Sources

**S1. `Cue::max_instances`'s doc** — *"a budget on plays, not a standing count of the voices sounding. The cap decides
whether the **next** play may start, and a voice it admits lives out its length."* Iteration 21's reading, and the
reason a lowered cap does not cut live voices.

**S2. `docs/guide/buses.md`** — *"`max_voices` is the whole table's budget; a bus's own `max_voices` caps it further
(`0` is the table's). A cue's `max_instances` is the same kind of number for one cue — and all three are **budgets on
plays**, not standing counts."* Three caps, one kind of number, and the guide already says so.

**S3. Gregory, *Game Engine Architecture* 3e §14.5.8.4** (vendored in `docs/research/siren.md`) — a group has its own
limit and the engine has another, kept apart. The crate's `Cap` enum is that split.

**S4. `CueProblem::UnlimitedInstances`** — *"`max_instances` is `0`: no per-cue limit, which the whole-table budget still
bounds."* So `x0` is not a display value; the row must say `all` and let `problems()` report it.

**S5. `Refusal::VoiceLimit { bus: Option<String> }`** — *"the bus that had no room, when one did"*; iteration 24 already
made the **refusal** name its cap. The **steal** did not, which is the same fact on the other side of the decision.

## The technique

1. The cue row gains a polyphony column beside `priority`: `x2`, `x1`, or `all` for the documented `0`.
2. A **`poly` step** asks `footstep.grass` three times in one frame. Against `max_instances: 2` that is one cue steal and
   two live voices — the row's `x2`, made true. It sits **last** in the default script so the still's log keeps the
   duck, the stinger and the spatial play above it.
3. `room_for` returns `Vec<(Entity, Cap)>` instead of `Vec<Entity>`, so the stop sentence can name the cap:
   `stolen by footstep.grass (cue)`, `(bus)`, `(table)`.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `max_instances` default | 1 | `Cue::one()` |
| `max_instances: 0` | no per-cue limit; the table's `max_voices` still bounds it | S4 |
| the three caps | cue, bus, table — **budgets on plays** | S1, S2 |
| a steal's sentence | `stolen by <cue> (<cap>)` | S5 (the refusal already names its cap) |
| the poly step | 3 requests, `x2` → 2 voices, 1 cue steal | S1 |

## What good sounds like

A cue row that says `x2` beside `priority 10`, and a log whose steals say which number to raise:
`stolen by footstep.grass (cue)` when the cue is full, `stolen by sword.hit (bus)` when the bus is. The isolated `poly`
run shows **one** `(cue)` line and Effects at `2 / 2 voices` — the row and the log stating the same cap.
