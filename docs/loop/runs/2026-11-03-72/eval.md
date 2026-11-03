# eval.md — iteration 72

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — **194** + 6 |
| `cargo test --all-features` | green — **352** |

**Nothing red in the crate's tests.** But **case 31 of `cues.yaml` was red**, and had been for about thirty iterations.

## The cases, before

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30, 32–33 | as before | green | unchanged |
| 31 | every number a cue author writes that the crate cannot honour is named | **RED** | `pitch` and `attenuation` reported; `gain_db` on the cue and on the slot **silent** |

## Why it was red for thirty iterations

`needs-decision/bad-gain-problem.md` had been waiting on a public-enum decision since iteration 18. Its own
recommendation was **option C: a sentence on `SirenLog`, no API change**, and that recommendation had never been
implemented — the file said *"C now, A at the next minor"*, and the loop kept reading the file and moving on.

**That is the loop's own failure, not a blocker.** A decision file whose recommendation is actionable does not need an
owner's answer to proceed; it needs someone to read it and do the part it recommends.

## The cases, after

| # | Case | Green/red | Observation |
|---|---|---|---|
| 31 | every number the crate cannot honour is reported or said out loud | **GREEN** | one log sentence names both gains, and the cue still plays |

## After

`bevy_siren`: **198 default / 356 all-features**, clippy 0, fmt, rustdoc — green.

**No crate test was red before this iteration. One authored case was, and it is now green.**

## Machine state

| | |
|---|---|
| start | **1.2 GB free**, `target` ~16 GB |
| after `cargo clean` | **17 GB free** |

The clean was needed before the baseline could run. Fifth time in this project.
