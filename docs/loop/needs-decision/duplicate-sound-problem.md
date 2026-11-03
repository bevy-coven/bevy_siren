# Should a cue listing one sound twice under a repeat-avoiding `Repeat` be a `CueProblem`?

**Status:** open — this iteration fixes the **behaviour**; the report waits.

## What the crate does now

`VariantPicker` refuses the last **keys** heard, so `sounds: [(sound: "a"), (sound: "a"), (sound: "b")]` alternates and
never plays `a` twice running. A cue with `["a", "a"]` and nothing else has one sound, so it plays `a` every time — which
is correct: the crate cannot invent a recording.

**What is still missing:** `problems()` says nothing about the duplicate. An author who copy-pasted a line believed they
had two recordings; the table has one, and the report — the one thing that exists to say so — is silent.

## Options

### A — `CueProblem::RepeatedSound { name: String, sound: String }`

- **Cost:** `CueProblem` is a **public enum without `#[non_exhaustive]`**. A new variant breaks every consumer that
  matches it exhaustively. This is the same cost as `bad-gain-problem.md`'s option A, and the two would ideally land in
  the same breaking release.
- **Benefit:** the seventh table mistake sits with the six; the Cauldron's panel draws it with no new code.

### B — report it only when `Repeat` is `AvoidRepeat` with `avoid > 0`

- **Cost:** a *conditional* problem, so two tables with the same duplicate list one sentence and not the other. A report
  that depends on another field is harder to reason about.
- **Benefit:** no false alarm for a cue that **means** to repeat one sound — a burst weapon with `Repeat::Shuffle` and one
  sample, which `cues.yaml` case 3 already has.

### C — nothing; the behaviour fix is enough

- **Cost:** an author with a duplicate never learns they are missing a recording. The cue sounds *worse* than they think
  (fewer variants), and the crate has the information.
- **Benefit:** no API change.

## Recommendation

**B now, A with the same release as the gain variant.** A duplicate is only a fault when the author asked for variation,
so the condition is the point rather than a complication — and `cues.yaml` case 3 (one variant, `Shuffle`) is the
counter-example that must stay silent. Adding the variant is the breaking part, and it belongs with the other one.

**What it costs a consumer today:** a duplicate under `AvoidRepeat` plays correctly (the fix) and is not reported (the
gap). A host that inspects `problems()` cannot see it.
