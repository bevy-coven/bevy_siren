# eval.md — iteration 40

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 119 + 5 |
| `cargo test --features serde` | green — 126 + 5 |
| `cargo test --features mixer` | green — 128 + 5 |
| `cargo test --doc` | skipped in run 1, run at the end: **5 green** |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **9.9 GB free (98 %)** — every capture below is a still or a 29-frame GIF |

**Nothing was red.** So the item came from `cues.yaml`'s cases and the iteration-39 lead, not from a failure.

## The cases

`docs/loop/cues.yaml` — every case, against the crate as it stood:

| # | Case | Green/red before | Observation | Why |
|---|---|---|---|---|
| 1 | one play, one `Played` | green | `tests/all.rs` | — |
| 2 | unknown cue refused | green | `UnknownCue` | — |
| 3 | cue with no sounds refused | green | `NoSounds` | — |
| 4 | a missing sound refused, named | green | `MissingSound { sound }` | — |
| 5 | unknown bus refused | green | `UnknownBus { bus }` | — |
| 6 | voice limit refuses, names the bus | green | `VoiceLimit { bus }` | — |
| 7 | unstealable priority | green | — | — |
| 8 | no immediate repeat | green | `Repeat::AvoidRepeat` | — |
| 9 | duck fires on a source voice | green | `docs/guide/buses.md` | — |
| 10 | a duck recovers | green | — | — |
| 11 | two ducks do not stack, the deeper wins | green | `active_duck_db` | — |
| 12 | a stinger ducks the music | green in `tests/music.rs` | — | **but the example never fired one** |
| 13 | **the panel shows a duck's cause** | **RED** | `duck -8.0 dB by stinger` where −8.0 is `dialogue.greeting`'s | the row printed one entry's `by` beside another rule's depth; masked by there being only ever one duck on a bus |
| 14 | the refusal's log line | green | iteration 39 | — |

**Case 12 red in the example, case 13 red.** Both are the same item: `Stinger` was never written, so the first bus to see
two ducks at once was never rendered.

## Ranked, for `plan.md`

1. **Case 13, the misnamed duck** — a panel fact that is *wrong*, not missing: it sends a reader to the wrong rule. And
   case 12 behind it, the feature that reveals it.
2. A `Stopped` entry for a parent's despawn — blocked on the public-enum decision seven items wait on.
3. A `max_instances` row on the cue list — the table's budget is invisible.
4. The `Music` header row naming its live layers — `calm/combat` is shown, the **count** is not.
5. No clip for the crate's own `examples/render` stinger.

## After

All green, `cues.yaml` case 13 now green in all three of its shapes (one rule, one recovering, two overlapping). See
`review.md` for the three verdicts.
