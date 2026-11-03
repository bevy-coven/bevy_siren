# plan.md — iteration 21: the wrapper that cannot see a project's own buses

## The pick, and why it beats the runner-up

Iteration 20's lead (`serde`) came back **clean**: the round trip is exact, and an unknown field is refused by name. The
probe found two things instead, and the second is rank 1.

**Pick:** `CueTable::problems()` is a convenience wrapper over `problems_with(&DEFAULT_BUSES)`. A project that lists its
own buses — fully supported, and what `answer`'s `UnknownBus` check, `update_mix` and `SirenSettings::problems()` all
read — gets **one false `UnknownBus` per cue**, and **the Cauldron's audio panel draws exactly this call**:

```
settings.bus_names()          ["Sfx", "Voices"]
table.problems()              ["cue `gunshot` plays on bus `Sfx`, which the mix table does not have"]
table.problems_in(&settings)  []
```

Beats the runner-up (a `CueProblem` for a non-finite gain, also found): that one is **blocked** on a public-enum
decision already filed in iteration 18; this one needs **no** decision, and it is a report that lies in the editor.

## The design

1. **`CueTable::problems_in(&SirenSettings)`** — the settings-aware call, reading `settings.bus_names()`, the one answer
   every other reader uses. Needs no API break: it is a **new method**, and `problems()` keeps its documented meaning
   (*"against the crate's own four buses"*).
2. **`problems()`' doc gains the warning**, so a reader who calls it learns what it assumes and what to call instead.
3. **The Cauldron's audio panel calls `problems_in(&read)`** — it already has the settings on the line above, and its own
   bus list is built from them two lines below. Fixed in `bevy_grimoire` as its own commit, per the crate's rules.

**Why not change `problems()` itself?** Its doc is explicit (*"the crate's own four"*) and a host with no settings at hand
(a test, a data check) wants exactly that. The fault is that the call with **no** settings is the one named `problems`,
so a caller takes it by default; the fix is a call that cannot be wrong plus a doc that says which is which.

## Fields

None. No `CueTable`, `SirenSettings` or `MusicScore` field. One new method on `CueTable`; no public type changed.

## Tests

- `problems_in_reads_the_tables_own_buses` — a custom `Sfx` bus: `problems()` reports `UnknownBus`, `problems_in` reports
  nothing, and **both agree** when the table lists no buses (so the new call is not a second rule).
- The **doctest** on `problems_in` is a real, compiled example showing both calls side by side.
- Every existing test unchanged.

## The carve

- **Not** the `CueProblem::BadGain` variant: blocked on `needs-decision/bad-gain-problem.md`; recorded there with this
  iteration's evidence, and in `cues.yaml` as **case 31, `state: red`**.
- **Not** changing `problems()`: documented behaviour, and a host without settings wants it.
