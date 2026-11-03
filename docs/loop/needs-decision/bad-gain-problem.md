# Should a gain that is not a number be a `CueProblem` and a `Refusal`?

**Status:** open — this iteration **normalises** the value (see `runs/2026-11-02-18/`); the messages wait.

## What the crate does now

A non-finite `gain_db` (on a cue, a sound slot, or a `Play` override) and a non-finite `Play::pitch` are read as **no
gain** and **no detune**: the play is audible at the level the cue actually authored, and `Played` reports the number the
sound is at. That is one rule, matching what `distance_gain` already does for a non-finite distance
(`docs/guide/spatial.md`), and it needs no API change.

**What is still missing:** a host that inspects `problems()` or `Refused` cannot tell that an author typed something the
crate had to neutralise.

## Options

### A — `CueProblem::BadGain { name: String, sound: Option<String> }`

`problems()` names the cue, and the slot when the fault is in one.

- **Cost:** `CueProblem` is a **public enum without `#[non_exhaustive]`**. A new variant breaks every consumer that
  matches it exhaustively — a compile error in a host's panel code, which is a hard break for a 0.1.x patch.
- **Benefit:** the seventh authoring mistake sits with the six; the Cauldron's panel draws it with no new code.

### A′ — the same, with `#[non_exhaustive]` added first

- **Cost:** adding `#[non_exhaustive]` is **also** a breaking change (every exhaustive `match` fails), plus it changes
  the shape of future versions.
- **Benefit:** every later variant is free.

### B — `Refusal::NotANumber { cue: String, field: &'static str }`

A `Play` override the crate cannot honour is refused, not neutralised.

- **Cost:** same exhaustive-enum break, and a **behaviour change**: a play that used to sound now refuses. Any game
  passing a computed gain that is occasionally `NaN` would lose the sound outright.
- **Benefit:** the strictest reading of "a bad cue is refused, never played wrong". If the owner prefers refusal to
  neutralisation, this is the coherent version of it.

### C — a sentence on `SirenLog`, as iteration 16 did for a one-shot layer

- **Cost:** no API change, but the information is in a log a `match`-based host does not read.
- **Benefit:** zero breakage; a person sees it in the panel.

## Recommendation

**C now, A at the next minor, B only with an explicit decision to change the play's outcome.** Normalising is the
behaviour this iteration ships, because it is the one the crate already commits to for a distance and because a lost
sound is worse than a mis-typed number. A log line makes the neutralisation visible to a person today; the `CueProblem`
variant is the right home and costs a breaking change that a patch release should not carry.

**What it costs a consumer today:** a game reading only `problems()` or `Refused` cannot see that a number was not a
number. It hears the cue at its authored level, which is what it asked for.

## If the owner picks A

The variant is one line in `problems_with` and one sentence:

```
cue `shot` has a gain that is not a number
sound `step_03` in cue `footstep.grass` has a gain that is not a number
```
