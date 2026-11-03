# eval.md — iteration 42

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 120 + 5 |
| `cargo test --features serde` | green — 127 + 5 |
| `cargo test --features mixer` | green — 129 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **7.9 GB free (99 %)** — every capture kept small |
| `gh issue list` | **no open issues** |

**Nothing was red.**

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–30 | as before | green | — |
| 7 | a third play takes the oldest back | green | **log could not say which cap** |
| 8 | a bus cap holds three requests in one frame | green | same |
| 31 | a non-finite gain reported by `problems()` | **RED** | **blocked** on `needs-decision/bad-gain-problem.md` |

**Case 31 was investigated this iteration and is genuinely blocked.** Every route to a report — a `CueProblem` variant,
a `Refusal` variant, a new `Entry` variant — breaks a public enum with no `#[non_exhaustive]` (`grep` confirms none in
`src/`). The decision file's own recommendation (*"C now, A at the next minor"*) names a log sentence as the unblocked
part, but a log sentence needs `Entry` too, so there is nothing to build without the decision.

## The item picked instead

The case table said green; the **panel** said nothing. `max_instances` — the field that separates a footstep from a
click — was not on the cue row, and the log's steal sentence named the cue but not the cap, so `cues.yaml` cases 7 and 8
were proven by tests and unreadable in the picture.

## After

| | before | after |
|---|---|---|
| the cue row | name, bus, mode, gain, priority | **+ `x2` / `x1` / `all`** |
| a cue-cap steal | `stolen by footstep.grass` | `stolen by footstep.grass **(cue)**` |
| a bus-cap steal | `stolen by sword.hit` | `stolen by sword.hit **(bus)**` |
| tests | 120 / 127 / 129 + 5 | **121 / 128 / 130 + 5** |

All green: build, every feature set, doctests, clippy, fmt, rustdoc, examples. See `review.md`.
