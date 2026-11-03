# eval.md — baseline and the `cues.yaml` table, iteration 25

## Baseline (recorded before any change, 2026-11-03, toolchain 1.99.0)

| Check | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | 109 + 5 doctests green |
| `cargo test --features serde` | 116 + 5 green |
| `cargo test --features mixer` | 117 + 5 green |
| `cargo test --doc` | 5 green |
| clippy / fmt / rustdoc / examples | green |

**Nothing red.** Disk 14 GB free (98 % — watched). No open issues. Iteration 24's run has its `review.md`.

## The `cues.yaml` table

31 cases; **case 31 is red by design** (blocked on the gain decision); the rest green. **No case runs `examples/mixer`**,
and no case has ever produced a panel picture — the harness panicked.

## The lead, and what it became

Iteration 24's lead was `examples/mixer`, "the one visible harness the loop has never inspected frame by frame". Run it:

```
SIREN_FRAMES=140 SIREN_SHOT=… cargo run --release --example mixer
  thread panicked at examples/mixer.rs:376:33:
  attempt to calculate the remainder with a divisor of zero
  (no still written)
```

`capture_config` built `keys` from `unwrap_or_default()`; unset gives `""`, `"".split(',')` gives one empty token, nothing
matches, `keys` is empty, and `stage_keys` evaluates `% c.keys.len()` — `% 0`. The harness that exists to produce evidence
**could not produce any** on its own documented command.

## Ranked

1. **`examples/mixer` panics on the doc's default command** — a crash in the only visible harness, blocking every
   visible change's evidence. **The pick**, unblocked.
2. `CueProblem` for an empty `sound` key (`needs-decision/empty-sound-key.md`).
3. `CueProblem` for a duplicate key (`needs-decision/duplicate-sound-problem.md`).
4. `CueProblem::BadGain` (`needs-decision/bad-gain-problem.md`; case 31 red).
5. `Refusal::NotALoop` (`needs-decision/not-a-loop-refusal.md`).

**Items 2–5 are blocked on one public-enum decision the owner has not answered** (iteration 24 asked).

## After the change

| command | after | green? |
|---|---|---|
| `SIREN_FRAMES` only | still written at 1280×720 and 1920×1080 | green (panicked before) |
| `SIREN_KEYS=bogus` | one line naming the typo, then the default script | green (panicked before) |
| `SIREN_KEYS=layer` | still written, unchanged | green |
| no environment | 20 s, no panic | green (panicked before) |
| every existing case, every rendered cue | unchanged; 21/21 WAVs byte-identical | green |
