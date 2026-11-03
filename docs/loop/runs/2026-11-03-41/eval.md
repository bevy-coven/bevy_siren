# eval.md — iteration 41

## Baseline, recorded before anything changed

| Step | Result |
|---|---|
| `cargo build` | green |
| `cargo test` | green — 119 + 5 |
| `cargo test --features serde` | green — 126 + 5 |
| `cargo test --features mixer` | green — 128 + 5 |
| `cargo test --doc --all-features` | green — 5 |
| `cargo clippy --all-targets --all-features -D warnings` | green |
| `cargo fmt --check` | green |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps --all-features` | green |
| `cargo build --examples` | green |
| `df -h ~` | **9.8 GB free (98 %)** |
| `gh issue list` | **no open issues** |

**Nothing was red.** The item came from the `cues.yaml` table against what the panel can *show*.

## The cases

Every case in `docs/loop/cues.yaml` is green in `tests/`. The ones that were **green and invisible** are what this
iteration ranked:

| # | Case | Tested | Shown in the panel, before |
|---|---|---|---|
| 1–12 | variants, pitch, gain, ducks, caps, refusals, unknown bus | green | yes, or a refusal is out of the panel's scope |
| 13 | spatial play with no listener refused | green | **no** — no spatial cue existed |
| 14 | distance decides the gain (0 / −6.02 dB) | green | **no** — no `.at(..)` call existed |
| 15 | a layer change lands on the bar | green | yes (Music block) |
| 16 | a stinger ducks the music | green | yes (iteration 40) |
| 17 | the seed replays the stream | green | n/a |
| 24–30 | refusal payloads, slot reuse, non-spatial position, bus list, stale stop | green | some |

**Cases 13 and 14 red in the harness, green in the tests.** A feature a consumer cannot see is a feature the loop has not
finished showing — the same shape as iterations 39 and 40, now on the crate's spatial path.

**A second finding, from making the first work:** `Entry::line` wrote a played gain at `{:.1}`, so the log could not show
the −6.0206 dB case 14 asserts, nor tell a distance from an authored −6.0 dB.

## Ranked, for `plan.md`

1. **Cases 13 + 14 in the panel** — rank 2 core behaviour, and the gain-precision mismatch behind it.
2. A `max_instances` column on the cue list — rank 3 ergonomics.
3. The `Music` header lacks a live-layer count.
4. The GIF samples at ~8 fps; a sub-120 ms fade falls between frames.
5. No panel capture of a `MissingSound` refusal.

## After

All green: 120 / 127 / 129 + 5 doctests, clippy, fmt, rustdoc, examples. Case 14's number is now on the panel and in the
log, at a precision where they agree. See `review.md` for the three verdicts.
