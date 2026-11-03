# eval.md — iteration 70

## Baseline, before anything changed

| Step | Result |
|---|---|
| `cargo build` | green — 42.5 s after a clean |
| `cargo test` | green — **181** + 6 |
| `cargo test --all-features` | green — **324** |

**Nothing red.** Working tree clean at `2af5529`.

## Machine state

| | |
|---|---|
| start | **3.4 GB free**, `target` 16 GB |
| after `cargo clean` | **18 GB free** (15.2 GiB removed) |
| when the mixer suite failed | **130 MB** — `rustc-LLVM ERROR: IO failure on output stream` |
| after a second clean | **18 GB free** (18.6 GiB removed) |

**Fourth disk failure in this project.** A debug build of this crate plus the full `bevy` dev-dependency is ~19 GB here,
and running more than two feature combinations in one iteration needs a clean between them. The `mixer` and
`all-features` counts below were taken after the second clean, so they are measurements of code, not of a truncated
build.

## The cases

| # | Case | Green/red | Observation |
|---|---|---|---|
| 1–33 | as before | green | unchanged |
| 31 | every unhonourable authored number named by `problems()` | **RED** | `gain_db` silent — blocked on the `CueProblem` enum decision |

## After

`bevy_siren`: 181 default / 190 serde / **339 all-features**, clippy 0, fmt, rustdoc — green.

## Why the disk note is in `eval.md` and not a footnote

Three suites failed this iteration **for reasons that read as code errors**: a link failure on `proc-macro2` (iteration
66), a release build dying in `rustc-LLVM`, and `failed to write lib.rmeta`. None was a defect. The skill's own note
says a full disk fails as `ld: write() failed, errno=28`; the same failure now appears from `rustc` directly, which is
worth recording so the next iteration recognises it in one second rather than one build.
