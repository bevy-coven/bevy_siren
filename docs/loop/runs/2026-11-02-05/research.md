# research.md — iteration 5: a refusal that names the cap that refused

## The item

`Refusal::VoiceLimit { bus: Option<String> }` is produced by one call site and always with the cue's bus, whatever
cap actually failed. `room_for` has **three** independent hard caps — the cue's own `max_instances`, its bus's voice
count, and the table's `max_voices` — and returns one uninformative `None` from all three. So:

| what happened | what the panel says |
|---|---|
| `max_voices: 1` (the whole table) was full | ``no voice free on bus `Effects` `` |
| the cue's own `max_instances: 1` was full | ``no voice free on bus `Effects` `` |
| the bus's own cap was full | ``no voice free on bus `Effects` `` |

Proved with a throwaway example: the first two both print that sentence, and in both the `Effects` bus had free
room — the cap was the table's and the cue's. A person reading the Cauldron's log tail goes to the bus row, finds it
at 0 of 32, and has been told something false.

This is the loop's highest-ranked class, a refusal that is **wrong rather than missing**, in a crate whose whole
premise is that a refusal names what is missing. The loop's own words: *"A refusal is green when it is the right
refusal."* Cases 7–9 in `cues.yaml` each assert a `VoiceLimit` occurred; none asserts the sentence is true.

## Sources

1. **Gregory, *Game Engine Architecture* 3e, §14.5.8.4 "Instance Limiting"** (the passage this crate already quotes;
   read in home-still in iteration 0, doc id `d0213a22…`, lines 23093–23129):

   > *"Instance limiting is sometimes applied differently to different groups of sounds. For example, we might specify
   > that we should play up to four guns simultaneously, hear up to three people speaking at a time and allow up to
   > five other sound effects to play at once…"*

   Three **different** limits, named separately, because they are three different things a person tunes. That is the
   whole argument: if a sound engine distinguishes a group's limit from the global one when a designer sets them, it
   must distinguish them when it refuses on one.

   > *"we must use them wisely"* — and wisdom about a limit requires knowing **which** limit.

2. **The crate's own rule R4** (`docs/research/siren.md`): *"Per-cue polyphony, per-bus cap and a table-wide budget are
   three hard caps… a request that cannot be made room in is `Refused { VoiceLimit }` with nothing spawned."* Three
   caps, one refusal type — and today, one sentence for all three.

3. **The crate's own rule R6**: *"a play is never silent: `Played` or `Refused`, both with the reason."* A reason that
   names the wrong subject is one step short of silent: it sends the reader to the wrong place with confidence.

4. **`tests/mix.rs` as the existing evidence.** `the_table_budget_refuses_rather_than_grow` builds `max_voices: 1` and
   asserts `matches!(r.reason, Refusal::VoiceLimit { .. })` — the wildcard is exactly the gap: the test could not see
   that the payload says `Effects` when the cap that failed was the table's.

## The technique

`room_for` already knows which cap it is testing at each of its three returns. Give it a way to say so, and use the
**existing** `Option<String>` the way its own doc describes.

The payload is `VoiceLimit { bus: Option<String> }`, documented as *"The bus that had no room, **when one did**"*. Read
literally that is exactly the right encoding:

| cap that failed | is a bus the subject? | payload | sentence |
|---|---|---|---|
| the table's `max_voices` | no — every bus may have room | `None` | `no voice free` |
| the cue's own `max_instances` | no — its bus may have room | `None` | `no voice free` |
| the bus's own `max_voices` | yes | `Some(bus)` | `no voice free on bus \`Effects\`` |

So the fix is: `room_for` returns `Result<Vec<Entity>, Cap>` with a private `Cap::{Cue, Bus, Table}`, and `answer`
maps `Cap::Bus` to `Some(bus)` and the other two to `None`. **No public type changes.** A consumer matching
`VoiceLimit { bus: Some(_) }` sees fewer of them and `VoiceLimit { bus: None }` sees more — which is the corrected
behaviour, and is why the change gets a `CHANGELOG` line and a test on both sides.

Today's code is worse than the table above suggests: both of the probe's failures — the table's budget and the cue's
own polyphony — reported `Some("Effects")` while the `Effects` bus had free room. The bus is named only when the bus
is the subject.

**Two things this does not do, and one of them goes to a decision file.**

- The sentence for `None` is true but not diagnostic (*"no voice free"* says the budget is out, not which of the two
  budgets). Making it name `max_voices` or `max_instances` needs the cap in the payload.
- A project that wants the refusal to say *which* cap, by name, needs a field or a variant — a public API change, so
  `needs-decision/refusal-names-its-cap.md` carries it, with the sentence each option would print.

What ships is what the type already promises: **a bus is named only when a bus is what refused.**

## Parameters the sources report

| parameter | value | source |
|---|---|---|
| the three caps | cue `max_instances`, bus `max_voices`, table `max_voices` | Gregory §14.5.8.4's per-group limits; crate rule R4 |
| the order they are tested | cue → bus → table | `room_for`'s documented order, unchanged |
| the sentence for a full table | "no voice free" | `Refusal::VoiceLimit { bus: None }`'s existing `Display` |
| the sentence for a full bus or cue | "no voice free on bus `Effects`" | existing `Display`, unchanged |
| **a cap's name in the payload** | proposed, not decided | `needs-decision/refusal-names-its-cap.md` |

No new number. The change is which of three already-decided facts reaches a person.

## What good sounds like

Nothing audible changes — no sample, gain or duck moves. What changes is what a person **reads**:

| situation | before | after |
|---|---|---|
| table budget full, bus `Effects` has 0 of 32 | `no voice free on bus \`Effects\`` | `no voice free` |
| bus `Effects` at its own 2-of-2 | `no voice free on bus \`Effects\`` | `no voice free on bus \`Effects\`` (unchanged, now true) |
| cue at its own `max_instances` | `no voice free on bus \`Effects\`` | `no voice free on bus \`Effects\`` (unchanged, still ambiguous — the decision file's subject) |

A reviewer can judge it without hearing it: two `Refused` messages with **different** text where today there is one —
`render.txt`'s `refusal-*.txt` files and a new probe in `examples/render` that forces each cap and writes each
sentence.
