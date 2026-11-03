# Iteration 80 — plan

## First: unblock the machine

Six iterations were blocked by `errno=28`, and the two before this one were absorbed by `cargo clean -p` on **dependency**
artifacts — which is the wrong lever and is what made each subsequent build need a fresh copy. The right lever, measured
this iteration:

| Path | Size | Regenerable? |
|---|---|---|
| `target/debug/examples` | **4.2 GB** | yes — this crate's own binaries |
| `target/debug/deps` (this crate's share) | ~2 GB | yes |
| `cargo clean -p bevy_siren` | **6.3 GB freed** | — |

`cargo clean -p bevy_siren` took the volume from **122 MB** to **6.2 GB** in one command, touching no dependency.
**That is the fix, and it belongs in the loop's machine facts** rather than being rediscovered.

## The ranked list

| # | Item | Why |
|---|---|---|
| **1** | **The failing ratchet** | Category 1 — a red test in `main` since `63ba157`. The loop's first rule is that an earlier iteration's half-done work is finished first. |
| 2 | The full baseline | Never completed in five iterations; everything else is judged against it. |
| 3 | Case 19 perf | Owed since iteration 76. |
| 4 | The visual gate | Owed since iteration 76. |
| 5 | `#16` incremental indexing | The next implementable p0, now that a compiler works. |

**Why (1) beats (5) in one sentence:** shipping a new feature on top of a red suite is the thing the loop exists to
prevent, and this red suite was *mine*.

## The change

One assertion in `tests/library.rs`: the pinned feature string gains `"dep:ebur128"`, and the comment above it gains
`ebur128`'s argument — **pure Rust with `default-features = false`**, no `cc`, no C object, 1.66 s build, agrees with
`ffmpeg` to 0.01 LU — so the two guards carry the same reasoning rather than one carrying it and the other citing it.

No `src/` change. No new dependency. No API change.

## Tests that prove it

`tests/library.rs::the_semantic_feature_declares_no_dependency_of_its_own` — the test itself, which was the failing one.
Its counterpart `tests/leaf.rs::the_dependency_list_stays_closed` already passed, and continues to.

## Carved off

Nothing. The item is one assertion, and the rest of the iteration is the measurement five iterations owed.
