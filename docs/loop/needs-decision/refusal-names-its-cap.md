# Does a refusal name the cap, or only the bus?

## The question

A `Refusal::VoiceLimit` now names the bus **only when the bus's own `max_voices` is what refused**. A full table budget
and a cue at its own `max_instances` both print *"no voice free"* — true, and less useful than *"the table's budget of
32 is full"* or *"`footstep.grass` is at its own 2"*. Should the refusal carry which cap failed, by name?

## The options

1. **Add a field: `VoiceLimit { bus: Option<String>, cap: Cap }`**, with `Cap::{Cue, Bus, Table}` public. Cost: a
   breaking change for any consumer matching the variant with named fields — the pattern the crate's own tests use is
   `VoiceLimit { .. }`, and the Cauldron does not match it at all (it prints `Entry::Refused`'s sentence), so the
   real-world cost is a host that destructures it. A `0.x` minor allows it. Benefit: the sentence becomes
   *"the table's budget of 32 is full"* / *"bus `Effects` is at its 2"* / *"`dialogue.greeting` is at its 1"* — every
   refusal tells a person exactly which line to edit. The bus field then becomes redundant (`cap == Bus` implies the
   cue's own bus), which argues for option 2.
2. **Replace the payload: `VoiceLimit { subject: LimitSubject }`**, with
   `LimitSubject::{Table { max: u16 }, Bus { name: String, max: u16 }, Cue { name: String, max: u16 }}`. Cost: a
   breaking change of the same size, and the `bus` key a host may read goes away. Benefit: no redundant field, and the
   **limit's value** is carried, so a panel can print "2 of 2" against the row it names — which is what a person
   actually opens the panel to see.
3. **Change only the sentence, keep the payload.** Cost: none. Benefit: a better message today. Limitation: the
   payload still cannot tell a host *which* cap, so a game that wants to react to "table full" versus "bus full"
   (raise the bus cap, mute a group) still cannot.
4. **New variant: `Refusal::TableBudget` and `Refusal::CueAtItsLimit`, keeping `VoiceLimit { bus }` for the bus.** Cost:
   an exhaustive `match` on `Refusal` in a consumer breaks — the same cost as 1 and 2, spread over three arms, and a
   panel must now render three sentences instead of one. Benefit: each variant carries only its own facts and a
   `match` cannot read a field that is `None` for its case.

## The recommendation

**Option 2, in the next minor version.** The crate's premise is that a refusal names what is missing; a *limit* has a
name and a value, and both belong in the refusal. Option 1 carries the redundancy and leaves the value out; option 3
is a message improvement that still cannot be acted on; option 4 triples the arms for no fact option 2 does not carry.
The sentence option 2 prints is the one a person needs:

```
the table's budget of 32 is full
bus `Effects` is at its 2
`dialogue.greeting` is at its 1
```

until then, option 3 is what iteration 5 shipped: `bus: None` when no bus is the subject, so the sentence is true, and
`Some(bus)` only when the bus is what refused.

## What is blocked

Nothing. Iteration 5 shipped the honest half — a bus is named only when a bus is what refused — which needed no public
change because `VoiceLimit::bus` is documented *"the bus that had no room, when one did"*. The diagnostic half (which
cap, and how full) waits on this answer. A game that wants to distinguish the three today can compare
`SirenMix::voices_on(bus)` against `SirenSettings::bus_voice_limit` and `max_voices` itself, which is a workaround, not
an API.
