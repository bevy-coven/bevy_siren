# Should a layer naming a cue that ends be a `Refusal`, or a log line?

**Status:** open — this iteration ships the enforcement and the log line; the message is a decision.

## The problem

`MusicScore::problems` says *"music layer `calm` names cue `hit`, which is not a loop, so it cannot hold a layer"*. The
plugin now enforces it: the layer is not started and the sentence goes into `SirenLog` once. A host reading `Refused`
sees **nothing**, so a game that inspects refusals to surface authoring mistakes cannot tell this from a score it never
had.

Every other authoring mistake a play can hit *is* a `Refusal`: `UnknownCue`, `NoSounds`, `MissingSound`, `UnknownBus`,
`VoiceLimit`, `SpatiallyMute`. This one is a mistake about the **score**, not about the play.

## Options

### A — a new `Refusal::NotALoop { layer: String, cue: String }`, written when the layer is skipped

Symmetric with `UnknownCue`: a host's one place to surface authoring mistakes covers scores too.

- **Cost:** a new variant is a **breaking change** for any `match` on `Refusal` a consumer writes without a wildcard
  (the crate has no `#[non_exhaustive]`). Every consumer must add an arm.
- **Benefit:** one path per feature; a panel can list it beside the other refusals.

### B — reuse `Refusal::UnknownCue { cue }`

No API change.

- **Cost:** **lies**. The cue exists; the *layer* is what is wrong. The message would name a cue the table has, which is
  exactly what `UnknownCatalogue`-style bugs teach a host to distrust.
- **Benefit:** none a log line does not already give.

### C — keep the log line only (this iteration)

- **Cost:** a host that reads `Refused` and nothing else cannot see it. The line is in `SirenLog`, which the panel reads,
  so a person sees it and a program inspecting `Refused` does not.
- **Benefit:** no API change; the sentence already exists and says exactly what is wrong.

### D — `CueProblem::NotALoop` plus a `problems()` call the plugin makes itself

- **Cost:** a second report of the same fact (the log line and the problem list), and `problems()` is a **host** call
  today — the plugin would need to decide when to run it, and a score edited every frame would re-report.
- **Benefit:** no message-type change, and a panel already renders `problems()`.

## Recommendation

**C now, A next major.** The enforcement must not wait for a message change, and the log line carries the true
sentence. When the crate next takes a breaking change, `A` makes the refusals complete: a host should be able to treat
`Refused` as *every* reason a sound did not play.

**What it costs a consumer today:** a game inspecting `Refused` alone will not see a one-shot layer; it must read
`SirenLog` (which the panel does) or call `MusicScore::problems` (which the Cauldron's audio panel already does).
