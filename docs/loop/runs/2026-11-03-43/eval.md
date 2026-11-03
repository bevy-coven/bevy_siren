# eval.md — iteration 43

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 121 + 5 |
| `cargo test --features serde` | green — 128 + 5 |
| `cargo test --features mixer` | green — 130 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **7.9 GB free (99 %)** |
| `gh issue list` | no open issues |

**Nothing was red.** The item came from a probe of the parenting path, not from the case table.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |

Case 31 re-checked: every route (a `CueProblem`, `Refusal` or `Entry` variant) breaks a public enum with no
`#[non_exhaustive]`. Nothing to build without the decision.

## The item found

The case table does not cover **`Play::on`**, and `cues.yaml` has no case for it. Probing it found that a parented voice's
despawn — the ordinary end of a sound on a vehicle — is the **one end path with no log line**, on both backends. The
existing test asserted the slot was freed and never read the log, so nothing caught it.

**Suggested case 32** (not added this iteration — `cues.yaml` is not rewritten except to add a case a new behaviour
needs, and the iteration's own tests carry it):

```
- id: 32
  cue: engine.idle
  authored: { bus: Effects, mode: Loop, play: { on: owner, then: "despawn the owner" } }
  behaviour: A sound whose owner is despawned stops and the log says which sound and why.
  observed_by: "SirenLog's last `Stopped` entry for the cue, and `SirenMix::voices_on` after the despawn."
  green_when: "One `Stopped` whose `why` is \"owner despawned\" and which names the cue; the bus counts one fewer voice; a game's own unparented despawn logs nothing."
  source: "`Play::on`'s doc — the parent's despawn is the ordinary end; crate rule R6 — every play's outcome is readable"
```

## After

| | before | after |
|---|---|---|
| a parented sound's end | **no line**, both backends | `0.68s stopped \`engine.idle\`: owner despawned` |
| the mixer path | slot freed, silent | one line, shared system |
| an unparented game despawn | silent | **still silent** |
| tests | 121 / 128 / 130 + 5 | **122 / 129 / 132 + 5** |
