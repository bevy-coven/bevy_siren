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

## After (`matrix.log`, the same script on the final tree)

Every command exits 0. The only change in the counts is the 12 tests this iteration added to `tests/semantic_golden.rs` (8 for `eval::centred` and `eval::separation`, 4 for the second set's
checked-in files and the rule that every golden set has what `--set` needs); a run without `--features semantic` does not compile that module in.

| command | before | after |
|---|---|---|
| `cargo build` | ok | ok |
| `cargo test` | 6 doc + 200 | 6 doc + **200** |
| `--features serde` | 209 | **209** |
| `--features semantic` | 109 lib + 505 | 109 lib + **517** |
| `--features mixer` | 210 | **210** |
| `--all-features` | 109 lib + 529 | 109 lib + **541** |
| `cargo test --doc` / `--features readme-example --test all readme::` | 6 / 1 | 6 / 1 |
| `cargo build --examples` (default and `--all-features`) | ok | ok |
| `cargo clippy --all-targets --all-features -- -D warnings` | clean | clean |
| `cargo fmt --check` | clean | clean |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` | clean | clean |

After the matrix one doc comment (`Floor::min_score`) was corrected; `cargo fmt --check`, rustdoc with warnings denied and `cargo test --doc` were re-run and are clean.

**The measurement this iteration was for** (`evidence/after/`, the guide's "The second library"): on 303 game sounds the model's zero-shot accuracy is **37.3 %** (ESC-50 87.8 %), hit@1 **31 %** (90 %), the right answers'
median cosine **0.191** (0.468), and **13 of 29 descriptions have a best result under 0.35** (0 of 50). No floor separates the answerable from the unanswerable on either library. `0.35` is withdrawn; the default stays `0.0`.

**Still open after this iteration**: the default-floor decision is the owner's (`needs-decision/default-floor.md`, recommendation unchanged in direction: keep `0.0`); `#35`'s third criterion (anchor pairs, a person's); `#32`–`#34` (the editor's); `#39`
(decided not to ship). No `consumer` issue was filed against this crate.
