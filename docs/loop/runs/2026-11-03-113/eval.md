# Iteration 113 — eval: the baseline, and what is already red

## Baseline, run on arrival (`baseline-matrix.log`; the matrix is `../2026-11-03-112/matrix.sh`, which cleans this crate's own artifacts whenever
free space drops under 5 GB — it did, three times, and free space never fell below 5.3 GB)

**Nothing is red.** Every command exits 0, and the counts are exactly iteration 112's *after* column:

| command | result |
|---|---|
| `cargo build` | ok |
| `cargo test` | 6 doc + **200** |
| `--features serde` | **209** |
| `--features semantic` | 109 lib + **505** |
| `--features mixer` | **210** |
| `--all-features` | 109 lib + **529** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean |

The one warning on every build — *"linker stderr: ld: __eh_frame section too large (max 16MB)"* from `examples/mixer` against the full umbrella — is not
this crate's code and was recorded in iteration 112.

## `cues.yaml`

All 33 cases are asserted by the test matrix above (`tests/` has a test per case; iteration 112 changed no audio code and the 21 rendered WAVs
were byte-identical to 111's). None is red.

## Half-done work

None. `git status` is clean at `878b849` (= `origin/main`). Every `runs/` directory has a `review.md` except `2026-11-03-78` (research-only, verdicts in the log)
and `-98` (one file written by `evalbench`, not an iteration), both recorded in iteration 112's `eval.md`. The upstream report in `../2026-11-03-112/` is a draft
for the owner to file and is not an open item of this loop.

## Open issues (`issue://`, re-read)

`#32`–`#34` the editor (`bevy_grimoire`'s loop; none is filed there with `consumer:bevy_siren`); `#35` open on criterion 3 (a person's anchor judgements);
`#39` decided not to ship (ADR 0005), left open for the owner. **No `consumer` issue is open**, so nothing outranks the loop's own backlog.

## The Cauldron's panel test

Not run, for the reason iteration 112 gave and nothing has changed: `bevy_cauldron` pins `bevy_siren` at git rev `82d227e` (216 commits behind), so the test
runs against that rev whatever this loop does, and its 14 GB target directory does not fit beside this crate's.

## The claims audit (the skill's new step 2 paragraph), by `grep` on `README.md`, `docs/guide/`, `src/`

Absolute statements — *can decode*, *is indexed*, *costs no*, *never fails*, *guarantee*, *every file* — were searched for and each one that names a
behaviour was read against a test. The decode, header, index and graph claims were the findings of iteration 112 and are fixed. What remains is
either a rule with a test (`library.rs`: a symlink out of the root is refused; the sidecar is the only source of provenance) or a statement about
arithmetic (`mix.rs`: the budget is a sample cost, not a headroom guarantee). **No new defect candidate outranks the second golden library.**
