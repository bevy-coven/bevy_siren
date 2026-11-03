# research.md — iteration 21: the report covers two of a cue's numbers and not the third

## The item

Iteration 20 left a lead: the `serde` round trip against a table that parses and is wrong. Probed, **serde is sound**:

```
gain_db: NaN in RON  ->  parses, gain = NaN
to_ron()             ->  writes `gain_db: NaN`
from_ron(to_ron(x))  ->  gain = NaN          (round trip exact)
unknown field `volume`  ->  refused, naming the field and the ones it accepts
max_instances: 65535    ->  parses, and `problems()` says nothing (no rule binds it to the table's budget)
```

So the lead was clean and the item is somewhere else — and it is one the same probe found:

```
cue `gain_db: NaN`      -> problems(): 0 sentences
slot `gain_db: NaN`     -> problems(): 0 sentences
pitch low > high        -> 1: "has pitch.low past pitch.high, so no detune can be drawn"
attenuation near_m: -1  -> 1: "has an attenuation constant that is negative or not a number"
```

**The same struct reports two of its three numbers and not the third.** `Cue::pitch` and `Cue::attenuation` are
authoring mistakes with sentences; `Cue::gain_db` and `SoundSlot::gain_db` are the same kind of mistake and are silent.

## Sources

**S1. `CueProblem`'s own vocabulary** (`src/cue.rs`) — `InvertedPitch`, `BadAttenuation`, `NoSounds`, `DuplicateName`,
`UnknownBus`, `UnlimitedInstances`. `BadAttenuation`'s own sentence is *"an attenuation constant that is **negative or not
a number**"* — the crate already treats a non-finite number as a reportable authoring mistake, in the very enum a gain
would join.

**S2. `model::finite_or` and `docs/guide/buses.md`** (iteration 18) — a non-finite caller value is the **neutral** value
for what it measures, so a cue at `gain_db: NaN` **sounds at 0 dB** and `Played.gain_db` reports `0.00`. The play is
right; the *report* is what has no line. R6 (*"everything a play decides is readable"*) is satisfied by `Played`;
what fails is the **authoring** half, R6's sibling in `siren.md`'s house style: a bad cue is named in `problems()`.

**S3. `docs/loop/needs-decision/bad-gain-problem.md`** (iteration 18) — the question is already written: a seventh
`CueProblem` variant, or a `Refusal` for a bad request, or a log line. It records that `CueProblem` is a **public enum
without `#[non_exhaustive]`**, so a variant breaks every consumer's `match`.

**S4. `docs/loop/cues.yaml`'s own header** — *"a case that fails after a change is a failure of that iteration"*. The
table is the loop's specification, and it had **no case** for a cue number the crate cannot honour on the gain side.

## The second finding, and the one that shipped

The same probe, run against a project with **its own buses**, showed `CueTable::problems()` — a wrapper over
`problems_with(&DEFAULT_BUSES)` — reporting `UnknownBus` for a cue the crate **accepts and mixes**:

```
settings.bus_names()          ["Sfx", "Voices"]
table.problems()              ["cue `gunshot` plays on bus `Sfx`, which the mix table does not have"]
table.problems_in(&settings)  []
```

`answer`'s `UnknownBus` check reads `bus_names()`; `update_mix` reads `bus_names()`; `SirenSettings::problems()` reads
`bus_names()` (iteration 20). `CueTable::problems()` is the one reader that does not, and **the Cauldron's audio panel
calls it** (`crates/bevy_cauldron/src/audio_panel.rs`). That is a report that cries wolf in the editor, which is the
rank-1 shape this loop keeps finding — and unlike the gain variant it needs **no** public-enum decision.

## The technique

Three things this iteration can do **without** the decision:

1. **Record the defect as a case.** `cues.yaml` gains case 31 with `state: red` and `blocked_on:` naming the decision
   file. The table is what the next iteration ranks from, so a red row is the honest way to carry a blocked item —
   and the loop's rules let the file grow for exactly this.
2. **Stop the guide over-promising.** `docs/guide/cues.md` listed the six checks; an author reads that list as *"these
   are the mistakes the crate catches"* and will believe a `NaN` gain is caught. It now says which number is not, why,
   and what to do until the variant exists.
3. **Name the boundary in the decision file**, so the owner sees that this is the **same** question already asked, with
   new evidence: two of three numbers in one struct are covered.

## Parameters the sources report

| Parameter | Value | Source |
|---|---|---|
| `BadAttenuation` covers "negative **or not a number**" | — | S1 |
| a non-finite gain is neutralised to 0 dB at play time | `0.0` | S2 |
| `CueProblem` is a public enum without `#[non_exhaustive]` | — | S3 |
| a red case is a legitimate row in the table | — | S4 |

## What good sounds like

A cue whose `gain_db` is `NaN` — on the cue or on a slot — produces **one sentence** in `problems()`, in the shape of
its neighbours (`"cue \`shot\` has a gain that is not a number"`). Until the owner picks, this iteration makes the
**absence** visible in the two places a person looks: the cue table of cases, and the guide.
